//! Computations on places -- field projections, going from mir::Place, and writing
//! into a place.
//! All high-level functions to write to memory work on places as destinations.

use std::convert::TryFrom;
use std::hash::Hash;

use rustc::hir;
use rustc::mir;
use rustc::ty::{self, Ty};
use rustc::ty::layout::{self, Size, Align, LayoutOf, TyLayout, HasDataLayout, VariantIdx};

use super::{
    GlobalId, AllocId, Allocation, Scalar, EvalResult, Pointer, PointerArithmetic,
    EvalContext, Machine, AllocMap, AllocationExtra,
    RawConst, Immediate, ImmTy, ScalarMaybeUndef, Operand, OpTy, MemoryKind
};

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct MemPlace<Tag=(), Id=AllocId> {
    /// A place may have an integral pointer for ZSTs, and since it might
    /// be turned back into a reference before ever being dereferenced.
    /// However, it may never be undef.
    pub ptr: Scalar<Tag, Id>,
    pub align: Align,
    /// Metadata for unsized places.  Interpretation is up to the type.
    /// Must not be present for sized types, but can be missing for unsized types
    /// (e.g., `extern type`).
    pub meta: Option<Scalar<Tag, Id>>,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Place<Tag=(), Id=AllocId> {
    /// A place referring to a value allocated in the `Memory` system.
    Ptr(MemPlace<Tag, Id>),

    /// To support alloc-free locals, we are able to write directly to a local.
    /// (Without that optimization, we'd just always be a `MemPlace`.)
    Local {
        frame: usize,
        local: mir::Local,
    },
}

#[derive(Copy, Clone, Debug)]
pub struct PlaceTy<'tcx, Tag=()> {
    place: Place<Tag>,
    pub layout: TyLayout<'tcx>,
}

impl<'tcx, Tag> ::std::ops::Deref for PlaceTy<'tcx, Tag> {
    type Target = Place<Tag>;
    #[inline(always)]
    fn deref(&self) -> &Place<Tag> {
        &self.place
    }
}

/// A MemPlace with its layout. Constructing it is only possible in this module.
#[derive(Copy, Clone, Debug)]
pub struct MPlaceTy<'tcx, Tag=()> {
    mplace: MemPlace<Tag>,
    pub layout: TyLayout<'tcx>,
}

impl<'tcx, Tag> ::std::ops::Deref for MPlaceTy<'tcx, Tag> {
    type Target = MemPlace<Tag>;
    #[inline(always)]
    fn deref(&self) -> &MemPlace<Tag> {
        &self.mplace
    }
}

impl<'tcx, Tag> From<MPlaceTy<'tcx, Tag>> for PlaceTy<'tcx, Tag> {
    #[inline(always)]
    fn from(mplace: MPlaceTy<'tcx, Tag>) -> Self {
        PlaceTy {
            place: Place::Ptr(mplace.mplace),
            layout: mplace.layout
        }
    }
}

impl MemPlace {
    #[inline]
    pub fn with_default_tag<Tag>(self) -> MemPlace<Tag>
        where Tag: Default
    {
        MemPlace {
            ptr: self.ptr.with_default_tag(),
            align: self.align,
            meta: self.meta.map(Scalar::with_default_tag),
        }
    }
}

impl<Tag> MemPlace<Tag> {
    #[inline]
    pub fn erase_tag(self) -> MemPlace
    {
        MemPlace {
            ptr: self.ptr.erase_tag(),
            align: self.align,
            meta: self.meta.map(Scalar::erase_tag),
        }
    }

    #[inline]
    pub fn with_tag(self, new_tag: Tag) -> Self
    {
        MemPlace {
            ptr: self.ptr.with_tag(new_tag),
            align: self.align,
            meta: self.meta,
        }
    }

    #[inline(always)]
    pub fn from_scalar_ptr(ptr: Scalar<Tag>, align: Align) -> Self {
        MemPlace {
            ptr,
            align,
            meta: None,
        }
    }

    /// Produces a Place that will error if attempted to be read from or written to
    #[inline(always)]
    pub fn null(cx: &impl HasDataLayout) -> Self {
        Self::from_scalar_ptr(Scalar::ptr_null(cx), Align::from_bytes(1).unwrap())
    }

    #[inline(always)]
    pub fn from_ptr(ptr: Pointer<Tag>, align: Align) -> Self {
        Self::from_scalar_ptr(ptr.into(), align)
    }

    #[inline(always)]
    pub fn to_scalar_ptr_align(self) -> (Scalar<Tag>, Align) {
        assert!(self.meta.is_none());
        (self.ptr, self.align)
    }

    /// metact the ptr part of the mplace
    #[inline(always)]
    pub fn to_ptr(self) -> EvalResult<'tcx, Pointer<Tag>> {
        // At this point, we forget about the alignment information --
        // the place has been turned into a reference, and no matter where it came from,
        // it now must be aligned.
        self.to_scalar_ptr_align().0.to_ptr()
    }

    /// Turn a mplace into a (thin or fat) pointer, as a reference, pointing to the same space.
    /// This is the inverse of `ref_to_mplace`.
    #[inline(always)]
    pub fn to_ref(self) -> Immediate<Tag> {
        match self.meta {
            None => Immediate::Scalar(self.ptr.into()),
            Some(meta) => Immediate::ScalarPair(self.ptr.into(), meta.into()),
        }
    }

    pub fn offset(
        self,
        offset: Size,
        meta: Option<Scalar<Tag>>,
        cx: &impl HasDataLayout,
    ) -> EvalResult<'tcx, Self> {
        Ok(MemPlace {
            ptr: self.ptr.ptr_offset(offset, cx)?,
            align: self.align.restrict_for_offset(offset),
            meta,
        })
    }
}

impl<'tcx, Tag> MPlaceTy<'tcx, Tag> {
    /// Produces a MemPlace that works for ZST but nothing else
    #[inline]
    pub fn dangling(layout: TyLayout<'tcx>, cx: &impl HasDataLayout) -> Self {
        MPlaceTy {
            mplace: MemPlace::from_scalar_ptr(
                Scalar::from_uint(layout.align.abi.bytes(), cx.pointer_size()),
                layout.align.abi
            ),
            layout
        }
    }

    #[inline]
    pub fn with_tag(self, new_tag: Tag) -> Self
    {
        MPlaceTy {
            mplace: self.mplace.with_tag(new_tag),
            layout: self.layout,
        }
    }

    #[inline]
    pub fn offset(
        self,
        offset: Size,
        meta: Option<Scalar<Tag>>,
        layout: TyLayout<'tcx>,
        cx: &impl HasDataLayout,
    ) -> EvalResult<'tcx, Self> {
        Ok(MPlaceTy {
            mplace: self.mplace.offset(offset, meta, cx)?,
            layout,
        })
    }

    #[inline]
    fn from_aligned_ptr(ptr: Pointer<Tag>, layout: TyLayout<'tcx>) -> Self {
        MPlaceTy { mplace: MemPlace::from_ptr(ptr, layout.align.abi), layout }
    }

    #[inline]
    pub(super) fn len(self, cx: &impl HasDataLayout) -> EvalResult<'tcx, u64> {
        if self.layout.is_unsized() {
            // We need to consult `meta` metadata
            match self.layout.ty.sty {
                ty::Slice(..) | ty::Str =>
                    return self.mplace.meta.unwrap().to_usize(cx),
                _ => bug!("len not supported on unsized type {:?}", self.layout.ty),
            }
        } else {
            // Go through the layout.  There are lots of types that support a length,
            // e.g., SIMD types.
            match self.layout.fields {
                layout::FieldPlacement::Array { count, .. } => Ok(count),
                _ => bug!("len not supported on sized type {:?}", self.layout.ty),
            }
        }
    }

    #[inline]
    pub(super) fn vtable(self) -> EvalResult<'tcx, Pointer<Tag>> {
        match self.layout.ty.sty {
            ty::Dynamic(..) => self.mplace.meta.unwrap().to_ptr(),
            _ => bug!("vtable not supported on type {:?}", self.layout.ty),
        }
    }
}

impl<'tcx, Tag: ::std::fmt::Debug> OpTy<'tcx, Tag> {
    #[inline(always)]
    pub fn try_as_mplace(self) -> Result<MPlaceTy<'tcx, Tag>, Immediate<Tag>> {
        match self.op {
            Operand::Indirect(mplace) => Ok(MPlaceTy { mplace, layout: self.layout }),
            Operand::Immediate(imm) => Err(imm),
        }
    }

    #[inline(always)]
    pub fn to_mem_place(self) -> MPlaceTy<'tcx, Tag> {
        self.try_as_mplace().unwrap()
    }
}

impl<'tcx, Tag: ::std::fmt::Debug> Place<Tag> {
    /// Produces a Place that will error if attempted to be read from or written to
    #[inline(always)]
    pub fn null(cx: &impl HasDataLayout) -> Self {
        Place::Ptr(MemPlace::null(cx))
    }

    #[inline(always)]
    pub fn from_scalar_ptr(ptr: Scalar<Tag>, align: Align) -> Self {
        Place::Ptr(MemPlace::from_scalar_ptr(ptr, align))
    }

    #[inline(always)]
    pub fn from_ptr(ptr: Pointer<Tag>, align: Align) -> Self {
        Place::Ptr(MemPlace::from_ptr(ptr, align))
    }

    #[inline]
    pub fn to_mem_place(self) -> MemPlace<Tag> {
        match self {
            Place::Ptr(mplace) => mplace,
            _ => bug!("to_mem_place: expected Place::Ptr, got {:?}", self),

        }
    }

    #[inline]
    pub fn to_scalar_ptr_align(self) -> (Scalar<Tag>, Align) {
        self.to_mem_place().to_scalar_ptr_align()
    }

    #[inline]
    pub fn to_ptr(self) -> EvalResult<'tcx, Pointer<Tag>> {
        self.to_mem_place().to_ptr()
    }
}

impl<'tcx, Tag: ::std::fmt::Debug> PlaceTy<'tcx, Tag> {
    #[inline]
    pub fn to_mem_place(self) -> MPlaceTy<'tcx, Tag> {
        MPlaceTy { mplace: self.place.to_mem_place(), layout: self.layout }
    }
}

// separating the pointer tag for `impl Trait`, see https://github.com/rust-lang/rust/issues/54385
impl<'a, 'mir, 'tcx, Tag, M> EvalContext<'a, 'mir, 'tcx, M>
where
    // FIXME: Working around https://github.com/rust-lang/rust/issues/54385
    Tag: ::std::fmt::Debug+Default+Copy+Eq+Hash+'static,
    M: Machine<'a, 'mir, 'tcx, PointerTag=Tag>,
    // FIXME: Working around https://github.com/rust-lang/rust/issues/24159
    M::MemoryMap: AllocMap<AllocId, (MemoryKind<M::MemoryKinds>, Allocation<Tag, M::AllocExtra>)>,
    M::AllocExtra: AllocationExtra<Tag, M::MemoryExtra>,
{
    /// Take a value, which represents a (thin or fat) reference, and make it a place.
    /// Alignment is just based on the type.  This is the inverse of `MemPlace::to_ref()`.
    /// This does NOT call the "deref" machine hook, so it does NOT count as a
    /// deref as far as Stacked Borrows is concerned.  Use `deref_operand` for that!
    pub fn ref_to_mplace(
        &self,
        val: ImmTy<'tcx, M::PointerTag>,
    ) -> EvalResult<'tcx, MPlaceTy<'tcx, M::PointerTag>> {
        let pointee_type = val.layout.ty.builtin_deref(true).unwrap().ty;
        let layout = self.layout_of(pointee_type)?;

        let mplace = MemPlace {
            ptr: val.to_scalar_ptr()?,
            align: layout.align.abi,
            meta: val.to_meta()?,
        };
        Ok(MPlaceTy { mplace, layout })
    }

    // Take an operand, representing a pointer, and dereference it to a place -- that
    // will always be a MemPlace.  Lives in `place.rs` because it creates a place.
    // This calls the "deref" machine hook, and counts as a deref as far as
    // Stacked Borrows is concerned.
    pub fn deref_operand(
        &self,
        src: OpTy<'tcx, M::PointerTag>,
    ) -> EvalResult<'tcx, MPlaceTy<'tcx, M::PointerTag>> {
        let val = self.read_immediate(src)?;
        trace!("deref to {} on {:?}", val.layout.ty, *val);
        let mut place = self.ref_to_mplace(val)?;
        // Pointer tag tracking might want to adjust the tag.
        let mutbl = match val.layout.ty.sty {
            // `builtin_deref` considers boxes immutable, that's useless for our purposes
            ty::Ref(_, _, mutbl) => Some(mutbl),
            ty::Adt(def, _) if def.is_box() => Some(hir::MutMutable),
            ty::RawPtr(_) => None,
            _ => bug!("Unexpected pointer type {}", val.layout.ty.sty),
        };
        place.mplace.ptr = M::tag_dereference(self, place, mutbl)?;
        Ok(place)
    }

    /// Offset a pointer to project to a field. Unlike place_field, this is always
    /// possible without allocating, so it can take &self. Also return the field's layout.
    /// This supports both struct and array fields.
    #[inline(always)]
    pub fn mplace_field(
        &self,
        base: MPlaceTy<'tcx, M::PointerTag>,
        field: u64,
    ) -> EvalResult<'tcx, MPlaceTy<'tcx, M::PointerTag>> {
        // Not using the layout method because we want to compute on u64
        let offset = match base.layout.fields {
            layout::FieldPlacement::Arbitrary { ref offsets, .. } =>
                offsets[usize::try_from(field).unwrap()],
            layout::FieldPlacement::Array { stride, .. } => {
                let len = base.len(self)?;
                assert!(field < len, "Tried to access element {} of array/slice with length {}",
                    field, len);
                stride * field
            }
            layout::FieldPlacement::Union(count) => {
                assert!(field < count as u64,
                        "Tried to access field {} of union with {} fields", field, count);
                // Offset is always 0
                Size::from_bytes(0)
            }
        };
        // the only way conversion can fail if is this is an array (otherwise we already panicked
        // above). In that case, all fields are equal.
        let field_layout = base.layout.field(self, usize::try_from(field).unwrap_or(0))?;

        // Offset may need adjustment for unsized fields
        let (meta, offset) = if field_layout.is_unsized() {
            // re-use parent metadata to determine dynamic field layout
            let align = match self.size_and_align_of(base.meta, field_layout)? {
                Some((_, align)) => align,
                None if offset == Size::ZERO =>
                    // An extern type at offset 0, we fall back to its static alignment.
                    // FIXME: Once we have made decisions for how to handle size and alignment
                    // of `extern type`, this should be adapted.  It is just a temporary hack
                    // to get some code to work that probably ought to work.
                    field_layout.align.abi,
                None =>
                    bug!("Cannot compute offset for extern type field at non-0 offset"),
            };
            (base.meta, offset.align_to(align))
        } else {
            // base.meta could be present; we might be accessing a sized field of an unsized
            // struct.
            (None, offset)
        };

        // We do not look at `base.layout.align` nor `field_layout.align`, unlike
        // codegen -- mostly to see if we can get away with that
        base.offset(offset, meta, field_layout, self)
    }

    // Iterates over all fields of an array. Much more efficient than doing the
    // same by repeatedly calling `mplace_array`.
    pub fn mplace_array_fields(
        &self,
        base: MPlaceTy<'tcx, Tag>,
    ) ->
        EvalResult<'tcx, impl Iterator<Item=EvalResult<'tcx, MPlaceTy<'tcx, Tag>>> + 'a>
    {
        let len = base.len(self)?; // also asserts that we have a type where this makes sense
        let stride = match base.layout.fields {
            layout::FieldPlacement::Array { stride, .. } => stride,
            _ => bug!("mplace_array_fields: expected an array layout"),
        };
        let layout = base.layout.field(self, 0)?;
        let dl = &self.tcx.data_layout;
        Ok((0..len).map(move |i| base.offset(i * stride, None, layout, dl)))
    }

    pub fn mplace_subslice(
        &self,
        base: MPlaceTy<'tcx, M::PointerTag>,
        from: u64,
        to: u64,
    ) -> EvalResult<'tcx, MPlaceTy<'tcx, M::PointerTag>> {
        let len = base.len(self)?; // also asserts that we have a type where this makes sense
        assert!(from <= len - to);

        // Not using layout method because that works with usize, and does not work with slices
        // (that have count 0 in their layout).
        let from_offset = match base.layout.fields {
            layout::FieldPlacement::Array { stride, .. } =>
                stride * from,
            _ => bug!("Unexpected layout of index access: {:#?}", base.layout),
        };

        // Compute meta and new layout
        let inner_len = len - to - from;
        let (meta, ty) = match base.layout.ty.sty {
            // It is not nice to match on the type, but that seems to be the only way to
            // implement this.
            ty::Array(inner, _) =>
                (None, self.tcx.mk_array(inner, inner_len)),
            ty::Slice(..) => {
                let len = Scalar::from_uint(inner_len, self.pointer_size());
                (Some(len), base.layout.ty)
            }
            _ =>
                bug!("cannot subslice non-array type: `{:?}`", base.layout.ty),
        };
        let layout = self.layout_of(ty)?;
        base.offset(from_offset, meta, layout, self)
    }

    pub fn mplace_downcast(
        &self,
        base: MPlaceTy<'tcx, M::PointerTag>,
        variant: VariantIdx,
    ) -> EvalResult<'tcx, MPlaceTy<'tcx, M::PointerTag>> {
        // Downcasts only change the layout
        assert!(base.meta.is_none());
        Ok(MPlaceTy { layout: base.layout.for_variant(self, variant), ..base })
    }

    /// Project into an mplace
    pub fn mplace_projection(
        &self,
        base: MPlaceTy<'tcx, M::PointerTag>,
        proj_elem: &mir::PlaceElem<'tcx>,
    ) -> EvalResult<'tcx, MPlaceTy<'tcx, M::PointerTag>> {
        use rustc::mir::ProjectionElem::*;
        Ok(match *proj_elem {
            Field(field, _) => self.mplace_field(base, field.index() as u64)?,
            Downcast(_, variant) => self.mplace_downcast(base, variant)?,
            Deref => self.deref_operand(base.into())?,

            Index(local) => {
                let n = *self.frame().locals[local].access()?;
                let n_layout = self.layout_of(self.tcx.types.usize)?;
                let n = self.read_scalar(OpTy { op: n, layout: n_layout })?;
                let n = n.to_bits(self.tcx.data_layout.pointer_size)?;
                self.mplace_field(base, u64::try_from(n).unwrap())?
            }

            ConstantIndex {
                offset,
                min_length,
                from_end,
            } => {
                let n = base.len(self)?;
                assert!(n >= min_length as u64);

                let index = if from_end {
                    n - u64::from(offset)
                } else {
                    u64::from(offset)
                };

                self.mplace_field(base, index)?
            }

            Subslice { from, to } =>
                self.mplace_subslice(base, u64::from(from), u64::from(to))?,
        })
    }

    /// Get the place of a field inside the place, and also the field's type.
    /// Just a convenience function, but used quite a bit.
    /// This is the only projection that might have a side-effect: We cannot project
    /// into the field of a local `ScalarPair`, we have to first allocate it.
    pub fn place_field(
        &mut self,
        base: PlaceTy<'tcx, M::PointerTag>,
        field: u64,
    ) -> EvalResult<'tcx, PlaceTy<'tcx, M::PointerTag>> {
        // FIXME: We could try to be smarter and avoid allocation for fields that span the
        // entire place.
        let mplace = self.force_allocation(base)?;
        Ok(self.mplace_field(mplace, field)?.into())
    }

    pub fn place_downcast(
        &self,
        base: PlaceTy<'tcx, M::PointerTag>,
        variant: VariantIdx,
    ) -> EvalResult<'tcx, PlaceTy<'tcx, M::PointerTag>> {
        // Downcast just changes the layout
        Ok(match base.place {
            Place::Ptr(mplace) =>
                self.mplace_downcast(MPlaceTy { mplace, layout: base.layout }, variant)?.into(),
            Place::Local { .. } => {
                let layout = base.layout.for_variant(self, variant);
                PlaceTy { layout, ..base }
            }
        })
    }

    /// Project into a place
    pub fn place_projection(
        &mut self,
        base: PlaceTy<'tcx, M::PointerTag>,
        proj_elem: &mir::ProjectionElem<'tcx, mir::Local, Ty<'tcx>>,
    ) -> EvalResult<'tcx, PlaceTy<'tcx, M::PointerTag>> {
        use rustc::mir::ProjectionElem::*;
        Ok(match *proj_elem {
            Field(field, _) =>  self.place_field(base, field.index() as u64)?,
            Downcast(_, variant) => self.place_downcast(base, variant)?,
            Deref => self.deref_operand(self.place_to_op(base)?)?.into(),
            // For the other variants, we have to force an allocation.
            // This matches `operand_projection`.
            Subslice { .. } | ConstantIndex { .. } | Index(_) => {
                let mplace = self.force_allocation(base)?;
                self.mplace_projection(mplace, proj_elem)?.into()
            }
        })
    }

    /// Evaluate statics and promoteds to an `MPlace`.  Used to share some code between
    /// `eval_place` and `eval_place_to_op`.
    pub(super) fn eval_place_to_mplace(
        &self,
        mir_place: &mir::Place<'tcx>
    ) -> EvalResult<'tcx, MPlaceTy<'tcx, M::PointerTag>> {
        use rustc::mir::Place::*;
        Ok(match *mir_place {
            Promoted(ref promoted) => {
                let instance = self.frame().instance;
                self.const_eval_raw(GlobalId {
                    instance,
                    promoted: Some(promoted.0),
                })?
            }

            Static(ref static_) => {
                let ty = self.monomorphize(static_.ty, self.substs());
                let layout = self.layout_of(ty)?;
                let instance = ty::Instance::mono(*self.tcx, static_.def_id);
                let cid = GlobalId {
                    instance,
                    promoted: None
                };
                // Just create a lazy reference, so we can support recursive statics.
                // tcx takes are of assigning every static one and only one unique AllocId.
                // When the data here is ever actually used, memory will notice,
                // and it knows how to deal with alloc_id that are present in the
                // global table but not in its local memory: It calls back into tcx through
                // a query, triggering the CTFE machinery to actually turn this lazy reference
                // into a bunch of bytes.  IOW, statics are evaluated with CTFE even when
                // this EvalContext uses another Machine (e.g., in miri).  This is what we
                // want!  This way, computing statics works concistently between codegen
                // and miri: They use the same query to eventually obtain a `ty::Const`
                // and use that for further computation.
                let alloc = self.tcx.alloc_map.lock().intern_static(cid.instance.def_id());
                MPlaceTy::from_aligned_ptr(Pointer::from(alloc).with_default_tag(), layout)
            }

            _ => bug!("eval_place_to_mplace called on {:?}", mir_place),
        })
    }

    /// Compute a place.  You should only use this if you intend to write into this
    /// place; for reading, a more efficient alternative is `eval_place_for_read`.
    pub fn eval_place(
        &mut self,
        mir_place: &mir::Place<'tcx>
    ) -> EvalResult<'tcx, PlaceTy<'tcx, M::PointerTag>> {
        use rustc::mir::Place::*;
        let place = match *mir_place {
            Local(mir::RETURN_PLACE) => match self.frame().return_place {
                Some(return_place) =>
                    // We use our layout to verify our assumption; caller will validate
                    // their layout on return.
                    PlaceTy {
                        place: *return_place,
                        layout: self.layout_of_local(self.frame(), mir::RETURN_PLACE)?,
                    },
                None => return err!(InvalidNullPointerUsage),
            },
            Local(local) => PlaceTy {
                place: Place::Local {
                    frame: self.cur_frame(),
                    local,
                },
                layout: self.layout_of_local(self.frame(), local)?,
            },

            Projection(ref proj) => {
                let place = self.eval_place(&proj.base)?;
                self.place_projection(place, &proj.elem)?
            }

            _ => self.eval_place_to_mplace(mir_place)?.into(),
        };

        self.dump_place(place.place);
        Ok(place)
    }

    /// Write a scalar to a place
    pub fn write_scalar(
        &mut self,
        val: impl Into<ScalarMaybeUndef<M::PointerTag>>,
        dest: PlaceTy<'tcx, M::PointerTag>,
    ) -> EvalResult<'tcx> {
        self.write_immediate(Immediate::Scalar(val.into()), dest)
    }

    /// Write an immediate to a place
    #[inline(always)]
    pub fn write_immediate(
        &mut self,
        src: Immediate<M::PointerTag>,
        dest: PlaceTy<'tcx, M::PointerTag>,
    ) -> EvalResult<'tcx> {
        self.write_immediate_no_validate(src, dest)?;

        if M::enforce_validity(self) {
            // Data got changed, better make sure it matches the type!
            self.validate_operand(self.place_to_op(dest)?, vec![], None, /*const_mode*/false)?;
        }

        Ok(())
    }

    /// Write an immediate to a place.
    /// If you use this you are responsible for validating that things got copied at the
    /// right type.
    fn write_immediate_no_validate(
        &mut self,
        src: Immediate<M::PointerTag>,
        dest: PlaceTy<'tcx, M::PointerTag>,
    ) -> EvalResult<'tcx> {
        if cfg!(debug_assertions) {
            // This is a very common path, avoid some checks in release mode
            assert!(!dest.layout.is_unsized(), "Cannot write unsized data");
            match src {
                Immediate::Scalar(ScalarMaybeUndef::Scalar(Scalar::Ptr(_))) =>
                    assert_eq!(self.pointer_size(), dest.layout.size,
                        "Size mismatch when writing pointer"),
                Immediate::Scalar(ScalarMaybeUndef::Scalar(Scalar::Bits { size, .. })) =>
                    assert_eq!(Size::from_bytes(size.into()), dest.layout.size,
                        "Size mismatch when writing bits"),
                Immediate::Scalar(ScalarMaybeUndef::Undef) => {}, // undef can have any size
                Immediate::ScalarPair(_, _) => {
                    // FIXME: Can we check anything here?
                }
            }
        }
        trace!("write_immediate: {:?} <- {:?}: {}", *dest, src, dest.layout.ty);

        // See if we can avoid an allocation. This is the counterpart to `try_read_immediate`,
        // but not factored as a separate function.
        let mplace = match dest.place {
            Place::Local { frame, local } => {
                match *self.stack[frame].locals[local].access_mut()? {
                    Operand::Immediate(ref mut dest_val) => {
                        // Yay, we can just change the local directly.
                        *dest_val = src;
                        return Ok(());
                    },
                    Operand::Indirect(mplace) => mplace, // already in memory
                }
            },
            Place::Ptr(mplace) => mplace, // already in memory
        };
        let dest = MPlaceTy { mplace, layout: dest.layout };

        // This is already in memory, write there.
        self.write_immediate_to_mplace_no_validate(src, dest)
    }

    /// Write an immediate to memory.
    /// If you use this you are responsible for validating that things git copied at the
    /// right type.
    fn write_immediate_to_mplace_no_validate(
        &mut self,
        value: Immediate<M::PointerTag>,
        dest: MPlaceTy<'tcx, M::PointerTag>,
    ) -> EvalResult<'tcx> {
        let (ptr, ptr_align) = dest.to_scalar_ptr_align();
        // Note that it is really important that the type here is the right one, and matches the
        // type things are read at. In case `src_val` is a `ScalarPair`, we don't do any magic here
        // to handle padding properly, which is only correct if we never look at this data with the
        // wrong type.

        // Nothing to do for ZSTs, other than checking alignment
        if dest.layout.is_zst() {
            return self.memory.check_align(ptr, ptr_align);
        }

        // check for integer pointers before alignment to report better errors
        let ptr = ptr.to_ptr()?;
        self.memory.check_align(ptr.into(), ptr_align)?;
        let tcx = &*self.tcx;
        // FIXME: We should check that there are dest.layout.size many bytes available in
        // memory.  The code below is not sufficient, with enough padding it might not
        // cover all the bytes!
        match value {
            Immediate::Scalar(scalar) => {
                match dest.layout.abi {
                    layout::Abi::Scalar(_) => {}, // fine
                    _ => bug!("write_immediate_to_mplace: invalid Scalar layout: {:#?}",
                            dest.layout)
                }
                self.memory.get_mut(ptr.alloc_id)?.write_scalar(
                    tcx, ptr, scalar, dest.layout.size
                )
            }
            Immediate::ScalarPair(a_val, b_val) => {
                let (a, b) = match dest.layout.abi {
                    layout::Abi::ScalarPair(ref a, ref b) => (&a.value, &b.value),
                    _ => bug!("write_immediate_to_mplace: invalid ScalarPair layout: {:#?}",
                              dest.layout)
                };
                let (a_size, b_size) = (a.size(self), b.size(self));
                let b_offset = a_size.align_to(b.align(self).abi);
                let b_align = ptr_align.restrict_for_offset(b_offset);
                let b_ptr = ptr.offset(b_offset, self)?;

                self.memory.check_align(b_ptr.into(), b_align)?;

                // It is tempting to verify `b_offset` against `layout.fields.offset(1)`,
                // but that does not work: We could be a newtype around a pair, then the
                // fields do not match the `ScalarPair` components.

                self.memory
                    .get_mut(ptr.alloc_id)?
                    .write_scalar(tcx, ptr, a_val, a_size)?;
                self.memory
                    .get_mut(b_ptr.alloc_id)?
                    .write_scalar(tcx, b_ptr, b_val, b_size)
            }
        }
    }

    /// Copy the data from an operand to a place.  This does not support transmuting!
    /// Use `copy_op_transmute` if the layouts could disagree.
    #[inline(always)]
    pub fn copy_op(
        &mut self,
        src: OpTy<'tcx, M::PointerTag>,
        dest: PlaceTy<'tcx, M::PointerTag>,
    ) -> EvalResult<'tcx> {
        self.copy_op_no_validate(src, dest)?;

        if M::enforce_validity(self) {
            // Data got changed, better make sure it matches the type!
            self.validate_operand(self.place_to_op(dest)?, vec![], None, /*const_mode*/false)?;
        }

        Ok(())
    }

    /// Copy the data from an operand to a place.  This does not support transmuting!
    /// Use `copy_op_transmute` if the layouts could disagree.
    /// Also, if you use this you are responsible for validating that things git copied at the
    /// right type.
    fn copy_op_no_validate(
        &mut self,
        src: OpTy<'tcx, M::PointerTag>,
        dest: PlaceTy<'tcx, M::PointerTag>,
    ) -> EvalResult<'tcx> {
        debug_assert!(!src.layout.is_unsized() && !dest.layout.is_unsized(),
            "Cannot copy unsized data");
        // We do NOT compare the types for equality, because well-typed code can
        // actually "transmute" `&mut T` to `&T` in an assignment without a cast.
        assert!(src.layout.details == dest.layout.details,
            "Layout mismatch when copying!\nsrc: {:#?}\ndest: {:#?}", src, dest);

        // Let us see if the layout is simple so we take a shortcut, avoid force_allocation.
        let src = match self.try_read_immediate(src)? {
            Ok(src_val) => {
                // Yay, we got a value that we can write directly.
                return self.write_immediate_no_validate(src_val, dest);
            }
            Err(mplace) => mplace,
        };
        // Slow path, this does not fit into an immediate. Just memcpy.
        trace!("copy_op: {:?} <- {:?}: {}", *dest, src, dest.layout.ty);

        let dest = self.force_allocation(dest)?;
        let (src_ptr, src_align) = src.to_scalar_ptr_align();
        let (dest_ptr, dest_align) = dest.to_scalar_ptr_align();
        self.memory.copy(
            src_ptr, src_align,
            dest_ptr, dest_align,
            dest.layout.size, false
        )?;

        Ok(())
    }

    /// Copy the data from an operand to a place.  The layouts may disagree, but they must
    /// have the same size.
    pub fn copy_op_transmute(
        &mut self,
        src: OpTy<'tcx, M::PointerTag>,
        dest: PlaceTy<'tcx, M::PointerTag>,
    ) -> EvalResult<'tcx> {
        if src.layout.details == dest.layout.details {
            // Fast path: Just use normal `copy_op`
            return self.copy_op(src, dest);
        }
        // We still require the sizes to match
        debug_assert!(!src.layout.is_unsized() && !dest.layout.is_unsized(),
            "Cannot copy unsized data");
        assert!(src.layout.size == dest.layout.size,
            "Size mismatch when transmuting!\nsrc: {:#?}\ndest: {:#?}", src, dest);

        // The hard case is `ScalarPair`.  `src` is already read from memory in this case,
        // using `src.layout` to figure out which bytes to use for the 1st and 2nd field.
        // We have to write them to `dest` at the offsets they were *read at*, which is
        // not necessarily the same as the offsets in `dest.layout`!
        // Hence we do the copy with the source layout on both sides.  We also make sure to write
        // into memory, because if `dest` is a local we would not even have a way to write
        // at the `src` offsets; the fact that we came from a different layout would
        // just be lost.
        let dest = self.force_allocation(dest)?;
        self.copy_op_no_validate(
            src,
            PlaceTy::from(MPlaceTy { mplace: *dest, layout: src.layout }),
        )?;

        if M::enforce_validity(self) {
            // Data got changed, better make sure it matches the type!
            self.validate_operand(dest.into(), vec![], None, /*const_mode*/false)?;
        }

        Ok(())
    }

    /// Make sure that a place is in memory, and return where it is.
    /// If the place currently refers to a local that doesn't yet have a matching allocation,
    /// create such an allocation.
    /// This is essentially `force_to_memplace`.
    pub fn force_allocation(
        &mut self,
        place: PlaceTy<'tcx, M::PointerTag>,
    ) -> EvalResult<'tcx, MPlaceTy<'tcx, M::PointerTag>> {
        let mplace = match place.place {
            Place::Local { frame, local } => {
                match *self.stack[frame].locals[local].access()? {
                    Operand::Indirect(mplace) => mplace,
                    Operand::Immediate(value) => {
                        // We need to make an allocation.
                        // FIXME: Consider not doing anything for a ZST, and just returning
                        // a fake pointer?  Are we even called for ZST?

                        // We need the layout of the local.  We can NOT use the layout we got,
                        // that might e.g., be an inner field of a struct with `Scalar` layout,
                        // that has different alignment than the outer field.
                        let local_layout = self.layout_of_local(&self.stack[frame], local)?;
                        let ptr = self.allocate(local_layout, MemoryKind::Stack);
                        // We don't have to validate as we can assume the local
                        // was already valid for its type.
                        self.write_immediate_to_mplace_no_validate(value, ptr)?;
                        let mplace = ptr.mplace;
                        // Update the local
                        *self.stack[frame].locals[local].access_mut()? =
                            Operand::Indirect(mplace);
                        mplace
                    }
                }
            }
            Place::Ptr(mplace) => mplace
        };
        // Return with the original layout, so that the caller can go on
        Ok(MPlaceTy { mplace, layout: place.layout })
    }

    pub fn allocate(
        &mut self,
        layout: TyLayout<'tcx>,
        kind: MemoryKind<M::MemoryKinds>,
    ) -> MPlaceTy<'tcx, M::PointerTag> {
        if layout.is_unsized() {
            assert!(self.tcx.features().unsized_locals, "cannot alloc memory for unsized type");
            // FIXME: What should we do here? We should definitely also tag!
            MPlaceTy::dangling(layout, self)
        } else {
            let ptr = self.memory.allocate(layout.size, layout.align.abi, kind);
            let ptr = M::tag_new_allocation(self, ptr, kind);
            MPlaceTy::from_aligned_ptr(ptr, layout)
        }
    }

    pub fn write_discriminant_index(
        &mut self,
        variant_index: VariantIdx,
        dest: PlaceTy<'tcx, M::PointerTag>,
    ) -> EvalResult<'tcx> {
        match dest.layout.variants {
            layout::Variants::Single { index } => {
                assert_eq!(index, variant_index);
            }
            layout::Variants::Tagged { ref tag, .. } => {
                let adt_def = dest.layout.ty.ty_adt_def().unwrap();
                assert!(variant_index.as_usize() < adt_def.variants.len());
                let discr_val = adt_def
                    .discriminant_for_variant(*self.tcx, variant_index)
                    .val;

                // raw discriminants for enums are isize or bigger during
                // their computation, but the in-memory tag is the smallest possible
                // representation
                let size = tag.value.size(self);
                let shift = 128 - size.bits();
                let discr_val = (discr_val << shift) >> shift;

                let discr_dest = self.place_field(dest, 0)?;
                self.write_scalar(Scalar::from_uint(discr_val, size), discr_dest)?;
            }
            layout::Variants::NicheFilling {
                dataful_variant,
                ref niche_variants,
                niche_start,
                ..
            } => {
                assert!(
                    variant_index.as_usize() < dest.layout.ty.ty_adt_def().unwrap().variants.len(),
                );
                if variant_index != dataful_variant {
                    let niche_dest =
                        self.place_field(dest, 0)?;
                    let niche_value = variant_index.as_u32() - niche_variants.start().as_u32();
                    let niche_value = (niche_value as u128)
                        .wrapping_add(niche_start);
                    self.write_scalar(
                        Scalar::from_uint(niche_value, niche_dest.layout.size),
                        niche_dest
                    )?;
                }
            }
        }

        Ok(())
    }

    /// Every place can be read from, so we can turm them into an operand
    #[inline(always)]
    pub fn place_to_op(
        &self,
        place: PlaceTy<'tcx, M::PointerTag>
    ) -> EvalResult<'tcx, OpTy<'tcx, M::PointerTag>> {
        let op = match place.place {
            Place::Ptr(mplace) => {
                Operand::Indirect(mplace)
            }
            Place::Local { frame, local } =>
                *self.stack[frame].locals[local].access()?
        };
        Ok(OpTy { op, layout: place.layout })
    }

    pub fn raw_const_to_mplace(
        &self,
        raw: RawConst<'tcx>,
    ) -> EvalResult<'tcx, MPlaceTy<'tcx, M::PointerTag>> {
        // This must be an allocation in `tcx`
        assert!(self.tcx.alloc_map.lock().get(raw.alloc_id).is_some());
        let layout = self.layout_of(raw.ty)?;
        Ok(MPlaceTy::from_aligned_ptr(
            Pointer::new(raw.alloc_id, Size::ZERO).with_default_tag(),
            layout,
        ))
    }

    /// Turn a place with a `dyn Trait` type into a place with the actual dynamic type.
    /// Also return some more information so drop doesn't have to run the same code twice.
    pub(super) fn unpack_dyn_trait(&self, mplace: MPlaceTy<'tcx, M::PointerTag>)
    -> EvalResult<'tcx, (ty::Instance<'tcx>, MPlaceTy<'tcx, M::PointerTag>)> {
        let vtable = mplace.vtable()?; // also sanity checks the type
        let (instance, ty) = self.read_drop_type_from_vtable(vtable)?;
        let layout = self.layout_of(ty)?;

        // More sanity checks
        if cfg!(debug_assertions) {
            let (size, align) = self.read_size_and_align_from_vtable(vtable)?;
            assert_eq!(size, layout.size);
            // only ABI alignment is preserved
            assert_eq!(align, layout.align.abi);
        }

        let mplace = MPlaceTy {
            mplace: MemPlace { meta: None, ..*mplace },
            layout
        };
        Ok((instance, mplace))
    }
}
