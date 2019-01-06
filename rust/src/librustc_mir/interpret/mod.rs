//! An interpreter for MIR used in CTFE and by miri

mod cast;
mod eval_context;
mod place;
mod operand;
mod machine;
mod memory;
mod operator;
pub(crate) mod snapshot; // for const_eval
mod step;
mod terminator;
mod traits;
mod validity;
mod intrinsics;
mod visitor;

pub use rustc::mir::interpret::*; // have all the `interpret` symbols in one place: here

pub use self::eval_context::{
    EvalContext, Frame, StackPopCleanup, LocalValue,
};

pub use self::place::{Place, PlaceTy, MemPlace, MPlaceTy};

pub use self::memory::{Memory, MemoryKind};

pub use self::machine::{Machine, AllocMap, MayLeak};

pub use self::operand::{ScalarMaybeUndef, Immediate, ImmTy, Operand, OpTy};

pub use self::visitor::{ValueVisitor, MutValueVisitor};

pub use self::validity::RefTracking;
