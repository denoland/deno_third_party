use std::{fmt, env};

use hir::map::definitions::DefPathData;
use mir;
use ty::{self, Ty, layout};
use ty::layout::{Size, Align, LayoutError};
use rustc_target::spec::abi::Abi;

use super::{RawConst, Pointer, InboundsCheck, ScalarMaybeUndef};

use backtrace::Backtrace;

use ty::query::TyCtxtAt;
use errors::DiagnosticBuilder;

use syntax_pos::{Pos, Span};
use syntax::ast;
use syntax::symbol::Symbol;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ErrorHandled {
    /// Already reported a lint or an error for this evaluation
    Reported,
    /// Don't emit an error, the evaluation failed because the MIR was generic
    /// and the substs didn't fully monomorphize it.
    TooGeneric,
}

impl ErrorHandled {
    pub fn assert_reported(self) {
        match self {
            ErrorHandled::Reported => {},
            ErrorHandled::TooGeneric => bug!("MIR interpretation failed without reporting an error \
                                              even though it was fully monomorphized"),
        }
    }
}

pub type ConstEvalRawResult<'tcx> = Result<RawConst<'tcx>, ErrorHandled>;
pub type ConstEvalResult<'tcx> = Result<ty::Const<'tcx>, ErrorHandled>;

#[derive(Clone, Debug, RustcEncodable, RustcDecodable)]
pub struct ConstEvalErr<'tcx> {
    pub span: Span,
    pub error: ::mir::interpret::EvalErrorKind<'tcx, u64>,
    pub stacktrace: Vec<FrameInfo<'tcx>>,
}

#[derive(Clone, Debug, RustcEncodable, RustcDecodable)]
pub struct FrameInfo<'tcx> {
    pub call_site: Span, // this span is in the caller!
    pub instance: ty::Instance<'tcx>,
    pub lint_root: Option<ast::NodeId>,
}

impl<'tcx> fmt::Display for FrameInfo<'tcx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        ty::tls::with(|tcx| {
            if tcx.def_key(self.instance.def_id()).disambiguated_data.data
                == DefPathData::ClosureExpr
            {
                write!(f, "inside call to closure")?;
            } else {
                write!(f, "inside call to `{}`", self.instance)?;
            }
            if !self.call_site.is_dummy() {
                let lo = tcx.sess.source_map().lookup_char_pos_adj(self.call_site.lo());
                write!(f, " at {}:{}:{}", lo.filename, lo.line, lo.col.to_usize() + 1)?;
            }
            Ok(())
        })
    }
}

impl<'a, 'gcx, 'tcx> ConstEvalErr<'tcx> {
    pub fn struct_error(&self,
        tcx: TyCtxtAt<'a, 'gcx, 'tcx>,
        message: &str)
        -> Result<DiagnosticBuilder<'tcx>, ErrorHandled>
    {
        self.struct_generic(tcx, message, None)
    }

    pub fn report_as_error(&self,
        tcx: TyCtxtAt<'a, 'gcx, 'tcx>,
        message: &str
    ) -> ErrorHandled {
        let err = self.struct_error(tcx, message);
        match err {
            Ok(mut err) => {
                err.emit();
                ErrorHandled::Reported
            },
            Err(err) => err,
        }
    }

    pub fn report_as_lint(&self,
        tcx: TyCtxtAt<'a, 'gcx, 'tcx>,
        message: &str,
        lint_root: ast::NodeId,
    ) -> ErrorHandled {
        let lint = self.struct_generic(
            tcx,
            message,
            Some(lint_root),
        );
        match lint {
            Ok(mut lint) => {
                lint.emit();
                ErrorHandled::Reported
            },
            Err(err) => err,
        }
    }

    fn struct_generic(
        &self,
        tcx: TyCtxtAt<'a, 'gcx, 'tcx>,
        message: &str,
        lint_root: Option<ast::NodeId>,
    ) -> Result<DiagnosticBuilder<'tcx>, ErrorHandled> {
        match self.error {
            EvalErrorKind::Layout(LayoutError::Unknown(_)) |
            EvalErrorKind::TooGeneric => return Err(ErrorHandled::TooGeneric),
            EvalErrorKind::Layout(LayoutError::SizeOverflow(_)) |
            EvalErrorKind::TypeckError => return Err(ErrorHandled::Reported),
            _ => {},
        }
        trace!("reporting const eval failure at {:?}", self.span);
        let mut err = if let Some(lint_root) = lint_root {
            let node_id = self.stacktrace
                .iter()
                .rev()
                .filter_map(|frame| frame.lint_root)
                .next()
                .unwrap_or(lint_root);
            tcx.struct_span_lint_node(
                ::rustc::lint::builtin::CONST_ERR,
                node_id,
                tcx.span,
                message,
            )
        } else {
            struct_error(tcx, message)
        };
        err.span_label(self.span, self.error.to_string());
        // Skip the last, which is just the environment of the constant.  The stacktrace
        // is sometimes empty because we create "fake" eval contexts in CTFE to do work
        // on constant values.
        if self.stacktrace.len() > 0 {
            for frame_info in &self.stacktrace[..self.stacktrace.len()-1] {
                err.span_label(frame_info.call_site, frame_info.to_string());
            }
        }
        Ok(err)
    }
}

pub fn struct_error<'a, 'gcx, 'tcx>(
    tcx: TyCtxtAt<'a, 'gcx, 'tcx>,
    msg: &str,
) -> DiagnosticBuilder<'tcx> {
    struct_span_err!(tcx.sess, tcx.span, E0080, "{}", msg)
}

#[derive(Debug, Clone)]
pub struct EvalError<'tcx> {
    pub kind: EvalErrorKind<'tcx, u64>,
    pub backtrace: Option<Box<Backtrace>>,
}

impl<'tcx> EvalError<'tcx> {
    pub fn print_backtrace(&mut self) {
        if let Some(ref mut backtrace) = self.backtrace {
            print_backtrace(&mut *backtrace);
        }
    }
}

fn print_backtrace(backtrace: &mut Backtrace) {
    backtrace.resolve();
    eprintln!("\n\nAn error occurred in miri:\n{:?}", backtrace);
}

impl<'tcx> From<EvalErrorKind<'tcx, u64>> for EvalError<'tcx> {
    fn from(kind: EvalErrorKind<'tcx, u64>) -> Self {
        let backtrace = match env::var("RUST_CTFE_BACKTRACE") {
            // matching RUST_BACKTRACE, we treat "0" the same as "not present".
            Ok(ref val) if val != "0" => {
                let mut backtrace = Backtrace::new_unresolved();

                if val == "immediate" {
                    // Print it now
                    print_backtrace(&mut backtrace);
                    None
                } else {
                    Some(Box::new(backtrace))
                }
            },
            _ => None,
        };
        EvalError {
            kind,
            backtrace,
        }
    }
}

pub type AssertMessage<'tcx> = EvalErrorKind<'tcx, mir::Operand<'tcx>>;

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub enum EvalErrorKind<'tcx, O> {
    /// This variant is used by machines to signal their own errors that do not
    /// match an existing variant
    MachineError(String),

    FunctionAbiMismatch(Abi, Abi),
    FunctionArgMismatch(Ty<'tcx>, Ty<'tcx>),
    FunctionRetMismatch(Ty<'tcx>, Ty<'tcx>),
    FunctionArgCountMismatch,
    NoMirFor(String),
    UnterminatedCString(Pointer),
    DanglingPointerDeref,
    DoubleFree,
    InvalidMemoryAccess,
    InvalidFunctionPointer,
    InvalidBool,
    InvalidDiscriminant(ScalarMaybeUndef),
    PointerOutOfBounds {
        ptr: Pointer,
        check: InboundsCheck,
        allocation_size: Size,
    },
    InvalidNullPointerUsage,
    ReadPointerAsBytes,
    ReadBytesAsPointer,
    ReadForeignStatic,
    InvalidPointerMath,
    ReadUndefBytes(Size),
    DeadLocal,
    InvalidBoolOp(mir::BinOp),
    Unimplemented(String),
    DerefFunctionPointer,
    ExecuteMemory,
    BoundsCheck { len: O, index: O },
    Overflow(mir::BinOp),
    OverflowNeg,
    DivisionByZero,
    RemainderByZero,
    Intrinsic(String),
    InvalidChar(u128),
    StackFrameLimitReached,
    OutOfTls,
    TlsOutOfBounds,
    AbiViolation(String),
    AlignmentCheckFailed {
        required: Align,
        has: Align,
    },
    ValidationFailure(String),
    CalledClosureAsFunction,
    VtableForArgumentlessMethod,
    ModifiedConstantMemory,
    ModifiedStatic,
    AssumptionNotHeld,
    InlineAsm,
    TypeNotPrimitive(Ty<'tcx>),
    ReallocatedWrongMemoryKind(String, String),
    DeallocatedWrongMemoryKind(String, String),
    ReallocateNonBasePtr,
    DeallocateNonBasePtr,
    IncorrectAllocationInformation(Size, Size, Align, Align),
    Layout(layout::LayoutError<'tcx>),
    HeapAllocZeroBytes,
    HeapAllocNonPowerOfTwoAlignment(u64),
    Unreachable,
    Panic {
        msg: Symbol,
        line: u32,
        col: u32,
        file: Symbol,
    },
    ReadFromReturnPointer,
    PathNotFound(Vec<String>),
    UnimplementedTraitSelection,
    /// Abort in case type errors are reached
    TypeckError,
    /// Resolution can fail if we are in a too generic context
    TooGeneric,
    /// Cannot compute this constant because it depends on another one
    /// which already produced an error
    ReferencedConstant,
    GeneratorResumedAfterReturn,
    GeneratorResumedAfterPanic,
    InfiniteLoop,
}

pub type EvalResult<'tcx, T = ()> = Result<T, EvalError<'tcx>>;

impl<'tcx, O> EvalErrorKind<'tcx, O> {
    pub fn description(&self) -> &str {
        use self::EvalErrorKind::*;
        match *self {
            MachineError(ref inner) => inner,
            FunctionAbiMismatch(..) | FunctionArgMismatch(..) | FunctionRetMismatch(..)
            | FunctionArgCountMismatch =>
                "tried to call a function through a function pointer of incompatible type",
            InvalidMemoryAccess =>
                "tried to access memory through an invalid pointer",
            DanglingPointerDeref =>
                "dangling pointer was dereferenced",
            DoubleFree =>
                "tried to deallocate dangling pointer",
            InvalidFunctionPointer =>
                "tried to use a function pointer after offsetting it",
            InvalidBool =>
                "invalid boolean value read",
            InvalidDiscriminant(..) =>
                "invalid enum discriminant value read",
            PointerOutOfBounds { .. } =>
                "pointer offset outside bounds of allocation",
            InvalidNullPointerUsage =>
                "invalid use of NULL pointer",
            ValidationFailure(..) =>
                "type validation failed",
            ReadPointerAsBytes =>
                "a raw memory access tried to access part of a pointer value as raw bytes",
            ReadBytesAsPointer =>
                "a memory access tried to interpret some bytes as a pointer",
            ReadForeignStatic =>
                "tried to read from foreign (extern) static",
            InvalidPointerMath =>
                "attempted to do invalid arithmetic on pointers that would leak base addresses, \
                e.g., comparing pointers into different allocations",
            ReadUndefBytes(_) =>
                "attempted to read undefined bytes",
            DeadLocal =>
                "tried to access a dead local variable",
            InvalidBoolOp(_) =>
                "invalid boolean operation",
            Unimplemented(ref msg) => msg,
            DerefFunctionPointer =>
                "tried to dereference a function pointer",
            ExecuteMemory =>
                "tried to treat a memory pointer as a function pointer",
            BoundsCheck{..} =>
                "array index out of bounds",
            Intrinsic(..) =>
                "intrinsic failed",
            NoMirFor(..) =>
                "mir not found",
            InvalidChar(..) =>
                "tried to interpret an invalid 32-bit value as a char",
            StackFrameLimitReached =>
                "reached the configured maximum number of stack frames",
            OutOfTls =>
                "reached the maximum number of representable TLS keys",
            TlsOutOfBounds =>
                "accessed an invalid (unallocated) TLS key",
            AbiViolation(ref msg) => msg,
            AlignmentCheckFailed{..} =>
                "tried to execute a misaligned read or write",
            CalledClosureAsFunction =>
                "tried to call a closure through a function pointer",
            VtableForArgumentlessMethod =>
                "tried to call a vtable function without arguments",
            ModifiedConstantMemory =>
                "tried to modify constant memory",
            ModifiedStatic =>
                "tried to modify a static's initial value from another static's initializer",
            AssumptionNotHeld =>
                "`assume` argument was false",
            InlineAsm =>
                "miri does not support inline assembly",
            TypeNotPrimitive(_) =>
                "expected primitive type, got nonprimitive",
            ReallocatedWrongMemoryKind(_, _) =>
                "tried to reallocate memory from one kind to another",
            DeallocatedWrongMemoryKind(_, _) =>
                "tried to deallocate memory of the wrong kind",
            ReallocateNonBasePtr =>
                "tried to reallocate with a pointer not to the beginning of an existing object",
            DeallocateNonBasePtr =>
                "tried to deallocate with a pointer not to the beginning of an existing object",
            IncorrectAllocationInformation(..) =>
                "tried to deallocate or reallocate using incorrect alignment or size",
            Layout(_) =>
                "rustc layout computation failed",
            UnterminatedCString(_) =>
                "attempted to get length of a null terminated string, but no null found before end \
                of allocation",
            HeapAllocZeroBytes =>
                "tried to re-, de- or allocate zero bytes on the heap",
            HeapAllocNonPowerOfTwoAlignment(_) =>
                "tried to re-, de-, or allocate heap memory with alignment that is not a power of \
                two",
            Unreachable =>
                "entered unreachable code",
            Panic { .. } =>
                "the evaluated program panicked",
            ReadFromReturnPointer =>
                "tried to read from the return pointer",
            PathNotFound(_) =>
                "a path could not be resolved, maybe the crate is not loaded",
            UnimplementedTraitSelection =>
                "there were unresolved type arguments during trait selection",
            TypeckError =>
                "encountered constants with type errors, stopping evaluation",
            TooGeneric =>
                "encountered overly generic constant",
            ReferencedConstant =>
                "referenced constant has errors",
            Overflow(mir::BinOp::Add) => "attempt to add with overflow",
            Overflow(mir::BinOp::Sub) => "attempt to subtract with overflow",
            Overflow(mir::BinOp::Mul) => "attempt to multiply with overflow",
            Overflow(mir::BinOp::Div) => "attempt to divide with overflow",
            Overflow(mir::BinOp::Rem) => "attempt to calculate the remainder with overflow",
            OverflowNeg => "attempt to negate with overflow",
            Overflow(mir::BinOp::Shr) => "attempt to shift right with overflow",
            Overflow(mir::BinOp::Shl) => "attempt to shift left with overflow",
            Overflow(op) => bug!("{:?} cannot overflow", op),
            DivisionByZero => "attempt to divide by zero",
            RemainderByZero => "attempt to calculate the remainder with a divisor of zero",
            GeneratorResumedAfterReturn => "generator resumed after completion",
            GeneratorResumedAfterPanic => "generator resumed after panicking",
            InfiniteLoop =>
                "duplicate interpreter state observed here, const evaluation will never terminate",
        }
    }
}

impl<'tcx> fmt::Display for EvalError<'tcx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl<'tcx> fmt::Display for EvalErrorKind<'tcx, u64> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<'tcx, O: fmt::Debug> fmt::Debug for EvalErrorKind<'tcx, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::EvalErrorKind::*;
        match *self {
            PointerOutOfBounds { ptr, check, allocation_size } => {
                write!(f, "Pointer must be in-bounds{} at offset {}, but is outside bounds of \
                           allocation {} which has size {}",
                       match check {
                           InboundsCheck::Live => " and live",
                           InboundsCheck::MaybeDead => "",
                       },
                       ptr.offset.bytes(), ptr.alloc_id, allocation_size.bytes())
            },
            ValidationFailure(ref err) => {
                write!(f, "type validation failed: {}", err)
            }
            NoMirFor(ref func) => write!(f, "no mir for `{}`", func),
            FunctionAbiMismatch(caller_abi, callee_abi) =>
                write!(f, "tried to call a function with ABI {:?} using caller ABI {:?}",
                    callee_abi, caller_abi),
            FunctionArgMismatch(caller_ty, callee_ty) =>
                write!(f, "tried to call a function with argument of type {:?} \
                           passing data of type {:?}",
                    callee_ty, caller_ty),
            FunctionRetMismatch(caller_ty, callee_ty) =>
                write!(f, "tried to call a function with return type {:?} \
                           passing return place of type {:?}",
                    callee_ty, caller_ty),
            FunctionArgCountMismatch =>
                write!(f, "tried to call a function with incorrect number of arguments"),
            BoundsCheck { ref len, ref index } =>
                write!(f, "index out of bounds: the len is {:?} but the index is {:?}", len, index),
            ReallocatedWrongMemoryKind(ref old, ref new) =>
                write!(f, "tried to reallocate memory from {} to {}", old, new),
            DeallocatedWrongMemoryKind(ref old, ref new) =>
                write!(f, "tried to deallocate {} memory but gave {} as the kind", old, new),
            Intrinsic(ref err) =>
                write!(f, "{}", err),
            InvalidChar(c) =>
                write!(f, "tried to interpret an invalid 32-bit value as a char: {}", c),
            AlignmentCheckFailed { required, has } =>
               write!(f, "tried to access memory with alignment {}, but alignment {} is required",
                      has.bytes(), required.bytes()),
            TypeNotPrimitive(ty) =>
                write!(f, "expected primitive type, got {}", ty),
            Layout(ref err) =>
                write!(f, "rustc layout computation failed: {:?}", err),
            PathNotFound(ref path) =>
                write!(f, "Cannot find path {:?}", path),
            MachineError(ref inner) =>
                write!(f, "{}", inner),
            IncorrectAllocationInformation(size, size2, align, align2) =>
                write!(f, "incorrect alloc info: expected size {} and align {}, \
                           got size {} and align {}",
                    size.bytes(), align.bytes(), size2.bytes(), align2.bytes()),
            Panic { ref msg, line, col, ref file } =>
                write!(f, "the evaluated program panicked at '{}', {}:{}:{}", msg, file, line, col),
            InvalidDiscriminant(val) =>
                write!(f, "encountered invalid enum discriminant {}", val),
            _ => write!(f, "{}", self.description()),
        }
    }
}
