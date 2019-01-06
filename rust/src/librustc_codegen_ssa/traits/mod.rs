//! Interface of a Rust codegen backend
//!
//! This crate defines all the traits that have to be implemented by a codegen backend in order to
//! use the backend-agnostic codegen code in `rustc_codegen_ssa`.
//!
//! The interface is designed around two backend-specific data structures, the codegen context and
//! the builder. The codegen context is supposed to be read-only after its creation and during the
//! actual codegen, while the builder stores the information about the function during codegen and
//! is used to produce the instructions of the backend IR.
//!
//! Finaly, a third `Backend` structure has to implement methods related to how codegen information
//! is passed to the backend, especially for asynchronous compilation.
//!
//! The traits contain associated types that are backend-specific, such as the backend's value or
//! basic blocks.

mod abi;
mod asm;
mod backend;
mod builder;
mod consts;
mod debuginfo;
mod declare;
mod intrinsic;
mod misc;
mod statics;
mod type_;
mod write;

pub use self::abi::{AbiBuilderMethods, AbiMethods};
pub use self::asm::{AsmBuilderMethods, AsmMethods};
pub use self::backend::{Backend, BackendTypes, ExtraBackendMethods};
pub use self::builder::{BuilderMethods, OverflowOp};
pub use self::consts::ConstMethods;
pub use self::debuginfo::{DebugInfoBuilderMethods, DebugInfoMethods};
pub use self::declare::{DeclareMethods, PreDefineMethods};
pub use self::intrinsic::IntrinsicCallMethods;
pub use self::misc::MiscMethods;
pub use self::statics::{StaticMethods, StaticBuilderMethods};
pub use self::type_::{
    ArgTypeMethods, BaseTypeMethods, DerivedTypeMethods, LayoutTypeMethods, TypeMethods,
};
pub use self::write::{ModuleBufferMethods, ThinBufferMethods, WriteBackendMethods};

use std::fmt;

pub trait CodegenObject: Copy + PartialEq + fmt::Debug {}
impl<T: Copy + PartialEq + fmt::Debug> CodegenObject for T {}

pub trait CodegenMethods<'tcx>:
    Backend<'tcx>
    + TypeMethods<'tcx>
    + MiscMethods<'tcx>
    + ConstMethods<'tcx>
    + StaticMethods
    + DebugInfoMethods<'tcx>
    + AbiMethods<'tcx>
    + DeclareMethods<'tcx>
    + AsmMethods<'tcx>
    + PreDefineMethods<'tcx>
{
}

impl<'tcx, T> CodegenMethods<'tcx> for T where
    Self: Backend<'tcx>
        + TypeMethods<'tcx>
        + MiscMethods<'tcx>
        + ConstMethods<'tcx>
        + StaticMethods
        + DebugInfoMethods<'tcx>
        + AbiMethods<'tcx>
        + DeclareMethods<'tcx>
        + AsmMethods<'tcx>
        + PreDefineMethods<'tcx>
{
}

pub trait HasCodegen<'tcx>:
    Backend<'tcx> + ::std::ops::Deref<Target = <Self as HasCodegen<'tcx>>::CodegenCx>
{
    type CodegenCx: CodegenMethods<'tcx>
        + BackendTypes<
            Value = Self::Value,
            BasicBlock = Self::BasicBlock,
            Type = Self::Type,
            Funclet = Self::Funclet,
            DIScope = Self::DIScope,
        >;
}
