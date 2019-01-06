use std::fmt::Write;
use std::mem;

use syntax::source_map::{self, Span, DUMMY_SP};
use rustc::hir::def_id::DefId;
use rustc::hir::def::Def;
use rustc::mir;
use rustc::ty::layout::{
    self, Size, Align, HasDataLayout, LayoutOf, TyLayout
};
use rustc::ty::subst::{Subst, Substs};
use rustc::ty::{self, Ty, TyCtxt, TypeFoldable};
use rustc::ty::query::TyCtxtAt;
use rustc_data_structures::indexed_vec::IndexVec;
use rustc::mir::interpret::{
    ErrorHandled,
    GlobalId, Scalar, FrameInfo, AllocId,
    EvalResult, EvalErrorKind,
    truncate, sign_extend,
};
use rustc_data_structures::fx::FxHashMap;

use super::{
    Immediate, Operand, MemPlace, MPlaceTy, Place, PlaceTy, ScalarMaybeUndef,
    Memory, Machine
};

pub struct EvalContext<'a, 'mir, 'tcx: 'a + 'mir, M: Machine<'a, 'mir, 'tcx>> {
    /// Stores the `Machine` instance.
    pub machine: M,

    /// The results of the type checker, from rustc.
    pub tcx: TyCtxtAt<'a, 'tcx, 'tcx>,

    /// Bounds in scope for polymorphic evaluations.
    pub(crate) param_env: ty::ParamEnv<'tcx>,

    /// The virtual memory system.
    pub(crate) memory: Memory<'a, 'mir, 'tcx, M>,

    /// The virtual call stack.
    pub(crate) stack: Vec<Frame<'mir, 'tcx, M::PointerTag, M::FrameExtra>>,

    /// A cache for deduplicating vtables
    pub(super) vtables: FxHashMap<(Ty<'tcx>, Option<ty::PolyExistentialTraitRef<'tcx>>), AllocId>,
}

/// A stack frame.
#[derive(Clone)]
pub struct Frame<'mir, 'tcx: 'mir, Tag=(), Extra=()> {
    ////////////////////////////////////////////////////////////////////////////////
    // Function and callsite information
    ////////////////////////////////////////////////////////////////////////////////
    /// The MIR for the function called on this frame.
    pub mir: &'mir mir::Mir<'tcx>,

    /// The def_id and substs of the current function
    pub instance: ty::Instance<'tcx>,

    /// The span of the call site.
    pub span: source_map::Span,

    ////////////////////////////////////////////////////////////////////////////////
    // Return place and locals
    ////////////////////////////////////////////////////////////////////////////////
    /// Work to perform when returning from this function
    pub return_to_block: StackPopCleanup,

    /// The location where the result of the current stack frame should be written to,
    /// and its layout in the caller.
    pub return_place: Option<PlaceTy<'tcx, Tag>>,

    /// The list of locals for this stack frame, stored in order as
    /// `[return_ptr, arguments..., variables..., temporaries...]`.
    /// The locals are stored as `Option<Value>`s.
    /// `None` represents a local that is currently dead, while a live local
    /// can either directly contain `Scalar` or refer to some part of an `Allocation`.
    pub locals: IndexVec<mir::Local, LocalValue<Tag>>,

    ////////////////////////////////////////////////////////////////////////////////
    // Current position within the function
    ////////////////////////////////////////////////////////////////////////////////
    /// The block that is currently executed (or will be executed after the above call stacks
    /// return).
    pub block: mir::BasicBlock,

    /// The index of the currently evaluated statement.
    pub stmt: usize,

    /// Extra data for the machine
    pub extra: Extra,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum StackPopCleanup {
    /// Jump to the next block in the caller, or cause UB if None (that's a function
    /// that may never return). Also store layout of return place so
    /// we can validate it at that layout.
    Goto(Option<mir::BasicBlock>),
    /// Just do nohing: Used by Main and for the box_alloc hook in miri.
    /// `cleanup` says whether locals are deallocated.  Static computation
    /// wants them leaked to intern what they need (and just throw away
    /// the entire `ecx` when it is done).
    None { cleanup: bool },
}

// State of a local variable
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum LocalValue<Tag=(), Id=AllocId> {
    Dead,
    // Mostly for convenience, we re-use the `Operand` type here.
    // This is an optimization over just always having a pointer here;
    // we can thus avoid doing an allocation when the local just stores
    // immediate values *and* never has its address taken.
    Live(Operand<Tag, Id>),
}

impl<'tcx, Tag> LocalValue<Tag> {
    pub fn access(&self) -> EvalResult<'tcx, &Operand<Tag>> {
        match self {
            LocalValue::Dead => err!(DeadLocal),
            LocalValue::Live(ref val) => Ok(val),
        }
    }

    pub fn access_mut(&mut self) -> EvalResult<'tcx, &mut Operand<Tag>> {
        match self {
            LocalValue::Dead => err!(DeadLocal),
            LocalValue::Live(ref mut val) => Ok(val),
        }
    }
}

impl<'a, 'mir, 'tcx, M: Machine<'a, 'mir, 'tcx>> HasDataLayout
    for EvalContext<'a, 'mir, 'tcx, M>
{
    #[inline]
    fn data_layout(&self) -> &layout::TargetDataLayout {
        &self.tcx.data_layout
    }
}

impl<'a, 'mir, 'tcx, M> layout::HasTyCtxt<'tcx> for EvalContext<'a, 'mir, 'tcx, M>
    where M: Machine<'a, 'mir, 'tcx>
{
    #[inline]
    fn tcx<'d>(&'d self) -> TyCtxt<'d, 'tcx, 'tcx> {
        *self.tcx
    }
}

impl<'a, 'mir, 'tcx, M: Machine<'a, 'mir, 'tcx>> LayoutOf
    for EvalContext<'a, 'mir, 'tcx, M>
{
    type Ty = Ty<'tcx>;
    type TyLayout = EvalResult<'tcx, TyLayout<'tcx>>;

    #[inline]
    fn layout_of(&self, ty: Ty<'tcx>) -> Self::TyLayout {
        self.tcx.layout_of(self.param_env.and(ty))
            .map_err(|layout| EvalErrorKind::Layout(layout).into())
    }
}

impl<'a, 'mir, 'tcx: 'mir, M: Machine<'a, 'mir, 'tcx>> EvalContext<'a, 'mir, 'tcx, M> {
    pub fn new(
        tcx: TyCtxtAt<'a, 'tcx, 'tcx>,
        param_env: ty::ParamEnv<'tcx>,
        machine: M,
    ) -> Self {
        EvalContext {
            machine,
            tcx,
            param_env,
            memory: Memory::new(tcx),
            stack: Vec::new(),
            vtables: FxHashMap::default(),
        }
    }

    #[inline(always)]
    pub fn memory(&self) -> &Memory<'a, 'mir, 'tcx, M> {
        &self.memory
    }

    #[inline(always)]
    pub fn memory_mut(&mut self) -> &mut Memory<'a, 'mir, 'tcx, M> {
        &mut self.memory
    }

    #[inline(always)]
    pub fn stack(&self) -> &[Frame<'mir, 'tcx, M::PointerTag, M::FrameExtra>] {
        &self.stack
    }

    #[inline(always)]
    pub fn cur_frame(&self) -> usize {
        assert!(self.stack.len() > 0);
        self.stack.len() - 1
    }

    #[inline(always)]
    pub fn frame(&self) -> &Frame<'mir, 'tcx, M::PointerTag, M::FrameExtra> {
        self.stack.last().expect("no call frames exist")
    }

    #[inline(always)]
    pub fn frame_mut(&mut self) -> &mut Frame<'mir, 'tcx, M::PointerTag, M::FrameExtra> {
        self.stack.last_mut().expect("no call frames exist")
    }

    #[inline(always)]
    pub(super) fn mir(&self) -> &'mir mir::Mir<'tcx> {
        self.frame().mir
    }

    pub fn substs(&self) -> &'tcx Substs<'tcx> {
        if let Some(frame) = self.stack.last() {
            frame.instance.substs
        } else {
            Substs::empty()
        }
    }

    pub(super) fn resolve(
        &self,
        def_id: DefId,
        substs: &'tcx Substs<'tcx>
    ) -> EvalResult<'tcx, ty::Instance<'tcx>> {
        trace!("resolve: {:?}, {:#?}", def_id, substs);
        trace!("substs: {:#?}", self.substs());
        trace!("param_env: {:#?}", self.param_env);
        let substs = self.tcx.subst_and_normalize_erasing_regions(
            self.substs(),
            self.param_env,
            &substs,
        );
        ty::Instance::resolve(
            *self.tcx,
            self.param_env,
            def_id,
            substs,
        ).ok_or_else(|| EvalErrorKind::TooGeneric.into())
    }

    pub fn type_is_sized(&self, ty: Ty<'tcx>) -> bool {
        ty.is_sized(self.tcx, self.param_env)
    }

    pub fn type_is_freeze(&self, ty: Ty<'tcx>) -> bool {
        ty.is_freeze(*self.tcx, self.param_env, DUMMY_SP)
    }

    pub fn load_mir(
        &self,
        instance: ty::InstanceDef<'tcx>,
    ) -> EvalResult<'tcx, &'tcx mir::Mir<'tcx>> {
        // do not continue if typeck errors occurred (can only occur in local crate)
        let did = instance.def_id();
        if did.is_local()
            && self.tcx.has_typeck_tables(did)
            && self.tcx.typeck_tables_of(did).tainted_by_errors
        {
            return err!(TypeckError);
        }
        trace!("load mir {:?}", instance);
        match instance {
            ty::InstanceDef::Item(def_id) => {
                self.tcx.maybe_optimized_mir(def_id).ok_or_else(||
                    EvalErrorKind::NoMirFor(self.tcx.item_path_str(def_id)).into()
                )
            }
            _ => Ok(self.tcx.instance_mir(instance)),
        }
    }

    pub fn monomorphize<T: TypeFoldable<'tcx> + Subst<'tcx>>(
        &self,
        t: T,
        substs: &'tcx Substs<'tcx>
    ) -> T {
        // miri doesn't care about lifetimes, and will choke on some crazy ones
        // let's simply get rid of them
        let substituted = t.subst(*self.tcx, substs);
        self.tcx.normalize_erasing_regions(ty::ParamEnv::reveal_all(), substituted)
    }

    pub fn layout_of_local(
        &self,
        frame: &Frame<'mir, 'tcx, M::PointerTag, M::FrameExtra>,
        local: mir::Local
    ) -> EvalResult<'tcx, TyLayout<'tcx>> {
        let local_ty = frame.mir.local_decls[local].ty;
        let local_ty = self.monomorphize(local_ty, frame.instance.substs);
        self.layout_of(local_ty)
    }

    pub fn str_to_immediate(&mut self, s: &str) -> EvalResult<'tcx, Immediate<M::PointerTag>> {
        let ptr = self.memory.allocate_static_bytes(s.as_bytes()).with_default_tag();
        Ok(Immediate::new_slice(Scalar::Ptr(ptr), s.len() as u64, self))
    }

    /// Return the actual dynamic size and alignment of the place at the given type.
    /// Only the "meta" (metadata) part of the place matters.
    /// This can fail to provide an answer for extern types.
    pub(super) fn size_and_align_of(
        &self,
        metadata: Option<Scalar<M::PointerTag>>,
        layout: TyLayout<'tcx>,
    ) -> EvalResult<'tcx, Option<(Size, Align)>> {
        if !layout.is_unsized() {
            return Ok(Some((layout.size, layout.align.abi)));
        }
        match layout.ty.sty {
            ty::Adt(..) | ty::Tuple(..) => {
                // First get the size of all statically known fields.
                // Don't use type_of::sizing_type_of because that expects t to be sized,
                // and it also rounds up to alignment, which we want to avoid,
                // as the unsized field's alignment could be smaller.
                assert!(!layout.ty.is_simd());
                trace!("DST layout: {:?}", layout);

                let sized_size = layout.fields.offset(layout.fields.count() - 1);
                let sized_align = layout.align.abi;
                trace!(
                    "DST {} statically sized prefix size: {:?} align: {:?}",
                    layout.ty,
                    sized_size,
                    sized_align
                );

                // Recurse to get the size of the dynamically sized field (must be
                // the last field).  Can't have foreign types here, how would we
                // adjust alignment and size for them?
                let field = layout.field(self, layout.fields.count() - 1)?;
                let (unsized_size, unsized_align) = match self.size_and_align_of(metadata, field)? {
                    Some(size_and_align) => size_and_align,
                    None => {
                        // A field with extern type.  If this field is at offset 0, we behave
                        // like the underlying extern type.
                        // FIXME: Once we have made decisions for how to handle size and alignment
                        // of `extern type`, this should be adapted.  It is just a temporary hack
                        // to get some code to work that probably ought to work.
                        if sized_size == Size::ZERO {
                            return Ok(None)
                        } else {
                            bug!("Fields cannot be extern types, unless they are at offset 0")
                        }
                    }
                };

                // FIXME (#26403, #27023): We should be adding padding
                // to `sized_size` (to accommodate the `unsized_align`
                // required of the unsized field that follows) before
                // summing it with `sized_size`. (Note that since #26403
                // is unfixed, we do not yet add the necessary padding
                // here. But this is where the add would go.)

                // Return the sum of sizes and max of aligns.
                let size = sized_size + unsized_size;

                // Choose max of two known alignments (combined value must
                // be aligned according to more restrictive of the two).
                let align = sized_align.max(unsized_align);

                // Issue #27023: must add any necessary padding to `size`
                // (to make it a multiple of `align`) before returning it.
                //
                // Namely, the returned size should be, in C notation:
                //
                //   `size + ((size & (align-1)) ? align : 0)`
                //
                // emulated via the semi-standard fast bit trick:
                //
                //   `(size + (align-1)) & -align`

                Ok(Some((size.align_to(align), align)))
            }
            ty::Dynamic(..) => {
                let vtable = metadata.expect("dyn trait fat ptr must have vtable").to_ptr()?;
                // the second entry in the vtable is the dynamic size of the object.
                Ok(Some(self.read_size_and_align_from_vtable(vtable)?))
            }

            ty::Slice(_) | ty::Str => {
                let len = metadata.expect("slice fat ptr must have vtable").to_usize(self)?;
                let elem = layout.field(self, 0)?;
                Ok(Some((elem.size * len, elem.align.abi)))
            }

            ty::Foreign(_) => {
                Ok(None)
            }

            _ => bug!("size_and_align_of::<{:?}> not supported", layout.ty),
        }
    }
    #[inline]
    pub fn size_and_align_of_mplace(
        &self,
        mplace: MPlaceTy<'tcx, M::PointerTag>
    ) -> EvalResult<'tcx, Option<(Size, Align)>> {
        self.size_and_align_of(mplace.meta, mplace.layout)
    }

    pub fn push_stack_frame(
        &mut self,
        instance: ty::Instance<'tcx>,
        span: source_map::Span,
        mir: &'mir mir::Mir<'tcx>,
        return_place: Option<PlaceTy<'tcx, M::PointerTag>>,
        return_to_block: StackPopCleanup,
    ) -> EvalResult<'tcx> {
        if self.stack.len() > 1 { // FIXME should be "> 0", printing topmost frame crashes rustc...
            info!("PAUSING({}) {}", self.cur_frame(), self.frame().instance);
        }
        ::log_settings::settings().indentation += 1;

        // first push a stack frame so we have access to the local substs
        let extra = M::stack_push(self)?;
        self.stack.push(Frame {
            mir,
            block: mir::START_BLOCK,
            return_to_block,
            return_place,
            // empty local array, we fill it in below, after we are inside the stack frame and
            // all methods actually know about the frame
            locals: IndexVec::new(),
            span,
            instance,
            stmt: 0,
            extra,
        });

        // don't allocate at all for trivial constants
        if mir.local_decls.len() > 1 {
            // We put some marker immediate into the locals that we later want to initialize.
            // This can be anything except for LocalValue::Dead -- because *that* is the
            // value we use for things that we know are initially dead.
            let dummy =
                LocalValue::Live(Operand::Immediate(Immediate::Scalar(ScalarMaybeUndef::Undef)));
            let mut locals = IndexVec::from_elem(dummy, &mir.local_decls);
            // Return place is handled specially by the `eval_place` functions, and the
            // entry in `locals` should never be used. Make it dead, to be sure.
            locals[mir::RETURN_PLACE] = LocalValue::Dead;
            // Now mark those locals as dead that we do not want to initialize
            match self.tcx.describe_def(instance.def_id()) {
                // statics and constants don't have `Storage*` statements, no need to look for them
                Some(Def::Static(..)) | Some(Def::Const(..)) | Some(Def::AssociatedConst(..)) => {},
                _ => {
                    trace!("push_stack_frame: {:?}: num_bbs: {}", span, mir.basic_blocks().len());
                    for block in mir.basic_blocks() {
                        for stmt in block.statements.iter() {
                            use rustc::mir::StatementKind::{StorageDead, StorageLive};
                            match stmt.kind {
                                StorageLive(local) |
                                StorageDead(local) => {
                                    locals[local] = LocalValue::Dead;
                                }
                                _ => {}
                            }
                        }
                    }
                },
            }
            // Finally, properly initialize all those that still have the dummy value
            for (local, decl) in locals.iter_mut().zip(mir.local_decls.iter()) {
                match *local {
                    LocalValue::Live(_) => {
                        // This needs to be peoperly initialized.
                        let layout = self.layout_of(self.monomorphize(decl.ty, instance.substs))?;
                        *local = LocalValue::Live(self.uninit_operand(layout)?);
                    }
                    LocalValue::Dead => {
                        // Nothing to do
                    }
                }
            }
            // done
            self.frame_mut().locals = locals;
        }

        if self.stack.len() > 1 { // FIXME no check should be needed, but some instances ICE
            info!("ENTERING({}) {}", self.cur_frame(), self.frame().instance);
        }

        if self.stack.len() > self.tcx.sess.const_eval_stack_frame_limit {
            err!(StackFrameLimitReached)
        } else {
            Ok(())
        }
    }

    pub(super) fn pop_stack_frame(&mut self) -> EvalResult<'tcx> {
        if self.stack.len() > 1 { // FIXME no check should be needed, but some instances ICE
            info!("LEAVING({}) {}", self.cur_frame(), self.frame().instance);
        }
        ::log_settings::settings().indentation -= 1;
        let frame = self.stack.pop().expect(
            "tried to pop a stack frame, but there were none",
        );
        M::stack_pop(self, frame.extra)?;
        // Abort early if we do not want to clean up: We also avoid validation in that case,
        // because this is CTFE and the final value will be thoroughly validated anyway.
        match frame.return_to_block {
            StackPopCleanup::Goto(_) => {},
            StackPopCleanup::None { cleanup } => {
                if !cleanup {
                    assert!(self.stack.is_empty(), "only the topmost frame should ever be leaked");
                    // Leak the locals, skip validation.
                    return Ok(());
                }
            }
        }
        // Deallocate all locals that are backed by an allocation.
        for local in frame.locals {
            self.deallocate_local(local)?;
        }
        // Validate the return value. Do this after deallocating so that we catch dangling
        // references.
        if let Some(return_place) = frame.return_place {
            if M::enforce_validity(self) {
                // Data got changed, better make sure it matches the type!
                // It is still possible that the return place held invalid data while
                // the function is running, but that's okay because nobody could have
                // accessed that same data from the "outside" to observe any broken
                // invariant -- that is, unless a function somehow has a ptr to
                // its return place... but the way MIR is currently generated, the
                // return place is always a local and then this cannot happen.
                self.validate_operand(
                    self.place_to_op(return_place)?,
                    vec![],
                    None,
                    /*const_mode*/false,
                )?;
            }
        } else {
            // Uh, that shouldn't happen... the function did not intend to return
            return err!(Unreachable);
        }
        // Jump to new block -- *after* validation so that the spans make more sense.
        match frame.return_to_block {
            StackPopCleanup::Goto(block) => {
                self.goto_block(block)?;
            }
            StackPopCleanup::None { .. } => {}
        }

        if self.stack.len() > 1 { // FIXME should be "> 0", printing topmost frame crashes rustc...
            info!("CONTINUING({}) {}", self.cur_frame(), self.frame().instance);
        }

        Ok(())
    }

    /// Mark a storage as live, killing the previous content and returning it.
    /// Remember to deallocate that!
    pub fn storage_live(
        &mut self,
        local: mir::Local
    ) -> EvalResult<'tcx, LocalValue<M::PointerTag>> {
        assert!(local != mir::RETURN_PLACE, "Cannot make return place live");
        trace!("{:?} is now live", local);

        let layout = self.layout_of_local(self.frame(), local)?;
        let init = LocalValue::Live(self.uninit_operand(layout)?);
        // StorageLive *always* kills the value that's currently stored
        Ok(mem::replace(&mut self.frame_mut().locals[local], init))
    }

    /// Returns the old value of the local.
    /// Remember to deallocate that!
    pub fn storage_dead(&mut self, local: mir::Local) -> LocalValue<M::PointerTag> {
        assert!(local != mir::RETURN_PLACE, "Cannot make return place dead");
        trace!("{:?} is now dead", local);

        mem::replace(&mut self.frame_mut().locals[local], LocalValue::Dead)
    }

    pub(super) fn deallocate_local(
        &mut self,
        local: LocalValue<M::PointerTag>,
    ) -> EvalResult<'tcx> {
        // FIXME: should we tell the user that there was a local which was never written to?
        if let LocalValue::Live(Operand::Indirect(MemPlace { ptr, .. })) = local {
            trace!("deallocating local");
            let ptr = ptr.to_ptr()?;
            self.memory.dump_alloc(ptr.alloc_id);
            self.memory.deallocate_local(ptr)?;
        };
        Ok(())
    }

    pub fn const_eval_raw(
        &self,
        gid: GlobalId<'tcx>,
    ) -> EvalResult<'tcx, MPlaceTy<'tcx, M::PointerTag>> {
        let param_env = if self.tcx.is_static(gid.instance.def_id()).is_some() {
            ty::ParamEnv::reveal_all()
        } else {
            self.param_env
        };
        // We use `const_eval_raw` here, and get an unvalidated result.  That is okay:
        // Our result will later be validated anyway, and there seems no good reason
        // to have to fail early here.  This is also more consistent with
        // `Memory::get_static_alloc` which has to use `const_eval_raw` to avoid cycles.
        let val = self.tcx.const_eval_raw(param_env.and(gid)).map_err(|err| {
            match err {
                ErrorHandled::Reported => EvalErrorKind::ReferencedConstant,
                ErrorHandled::TooGeneric => EvalErrorKind::TooGeneric,
            }
        })?;
        self.raw_const_to_mplace(val)
    }

    pub fn dump_place(&self, place: Place<M::PointerTag>) {
        // Debug output
        if !log_enabled!(::log::Level::Trace) {
            return;
        }
        match place {
            Place::Local { frame, local } => {
                let mut allocs = Vec::new();
                let mut msg = format!("{:?}", local);
                if frame != self.cur_frame() {
                    write!(msg, " ({} frames up)", self.cur_frame() - frame).unwrap();
                }
                write!(msg, ":").unwrap();

                match self.stack[frame].locals[local].access() {
                    Err(err) => {
                        if let EvalErrorKind::DeadLocal = err.kind {
                            write!(msg, " is dead").unwrap();
                        } else {
                            panic!("Failed to access local: {:?}", err);
                        }
                    }
                    Ok(Operand::Indirect(mplace)) => {
                        let (ptr, align) = mplace.to_scalar_ptr_align();
                        match ptr {
                            Scalar::Ptr(ptr) => {
                                write!(msg, " by align({}) ref:", align.bytes()).unwrap();
                                allocs.push(ptr.alloc_id);
                            }
                            ptr => write!(msg, " by integral ref: {:?}", ptr).unwrap(),
                        }
                    }
                    Ok(Operand::Immediate(Immediate::Scalar(val))) => {
                        write!(msg, " {:?}", val).unwrap();
                        if let ScalarMaybeUndef::Scalar(Scalar::Ptr(ptr)) = val {
                            allocs.push(ptr.alloc_id);
                        }
                    }
                    Ok(Operand::Immediate(Immediate::ScalarPair(val1, val2))) => {
                        write!(msg, " ({:?}, {:?})", val1, val2).unwrap();
                        if let ScalarMaybeUndef::Scalar(Scalar::Ptr(ptr)) = val1 {
                            allocs.push(ptr.alloc_id);
                        }
                        if let ScalarMaybeUndef::Scalar(Scalar::Ptr(ptr)) = val2 {
                            allocs.push(ptr.alloc_id);
                        }
                    }
                }

                trace!("{}", msg);
                self.memory.dump_allocs(allocs);
            }
            Place::Ptr(mplace) => {
                match mplace.ptr {
                    Scalar::Ptr(ptr) => {
                        trace!("by align({}) ref:", mplace.align.bytes());
                        self.memory.dump_alloc(ptr.alloc_id);
                    }
                    ptr => trace!(" integral by ref: {:?}", ptr),
                }
            }
        }
    }

    pub fn generate_stacktrace(&self, explicit_span: Option<Span>) -> Vec<FrameInfo<'tcx>> {
        let mut last_span = None;
        let mut frames = Vec::new();
        for &Frame { instance, span, mir, block, stmt, .. } in self.stack().iter().rev() {
            // make sure we don't emit frames that are duplicates of the previous
            if explicit_span == Some(span) {
                last_span = Some(span);
                continue;
            }
            if let Some(last) = last_span {
                if last == span {
                    continue;
                }
            } else {
                last_span = Some(span);
            }
            let block = &mir.basic_blocks()[block];
            let source_info = if stmt < block.statements.len() {
                block.statements[stmt].source_info
            } else {
                block.terminator().source_info
            };
            let lint_root = match mir.source_scope_local_data {
                mir::ClearCrossCrate::Set(ref ivs) => Some(ivs[source_info.scope].lint_root),
                mir::ClearCrossCrate::Clear => None,
            };
            frames.push(FrameInfo { call_site: span, instance, lint_root });
        }
        trace!("generate stacktrace: {:#?}, {:?}", frames, explicit_span);
        frames
    }

    #[inline(always)]
    pub fn sign_extend(&self, value: u128, ty: TyLayout<'_>) -> u128 {
        assert!(ty.abi.is_signed());
        sign_extend(value, ty.size)
    }

    #[inline(always)]
    pub fn truncate(&self, value: u128, ty: TyLayout<'_>) -> u128 {
        truncate(value, ty.size)
    }
}
