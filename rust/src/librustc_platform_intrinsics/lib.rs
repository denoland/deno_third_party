#![allow(nonstandard_style)]

#![feature(nll)]

pub struct Intrinsic {
    pub inputs: &'static [&'static Type],
    pub output: &'static Type,

    pub definition: IntrinsicDef,
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub enum Type {
    Void,
    Integer(/* signed */ bool, u8, /* llvm width */ u8),
    Float(u8),
    Pointer(&'static Type, Option<&'static Type>, /* const */ bool),
    Vector(&'static Type, Option<&'static Type>, u16),
    Aggregate(bool, &'static [&'static Type]),
}

pub enum IntrinsicDef {
    Named(&'static str),
}

static I8: Type = Type::Integer(true, 8, 8);
static I16: Type = Type::Integer(true, 16, 16);
static I32: Type = Type::Integer(true, 32, 32);
static I64: Type = Type::Integer(true, 64, 64);
static U8: Type = Type::Integer(false, 8, 8);
static U16: Type = Type::Integer(false, 16, 16);
static U32: Type = Type::Integer(false, 32, 32);
static U64: Type = Type::Integer(false, 64, 64);
static F32: Type = Type::Float(32);
static F64: Type = Type::Float(64);

static I32_8: Type = Type::Integer(true, 32, 8);

static I8x8: Type = Type::Vector(&I8, None, 8);
static U8x8: Type = Type::Vector(&U8, None, 8);
static I8x16: Type = Type::Vector(&I8, None, 16);
static U8x16: Type = Type::Vector(&U8, None, 16);
static I8x32: Type = Type::Vector(&I8, None, 32);
static U8x32: Type = Type::Vector(&U8, None, 32);
static I8x64: Type = Type::Vector(&I8, None, 64);
static U8x64: Type = Type::Vector(&U8, None, 64);
static I8x128: Type = Type::Vector(&I8, None, 128);
static U8x128: Type = Type::Vector(&U8, None, 128);
static I8x256: Type = Type::Vector(&I8, None, 256);
static U8x256: Type = Type::Vector(&U8, None, 256);

static I16x4: Type = Type::Vector(&I16, None, 4);
static U16x4: Type = Type::Vector(&U16, None, 4);
static I16x8: Type = Type::Vector(&I16, None, 8);
static U16x8: Type = Type::Vector(&U16, None, 8);
static I16x16: Type = Type::Vector(&I16, None, 16);
static U16x16: Type = Type::Vector(&U16, None, 16);
static I16x32: Type = Type::Vector(&I16, None, 32);
static U16x32: Type = Type::Vector(&U16, None, 32);
static I16x64: Type = Type::Vector(&I16, None, 64);
static U16x64: Type = Type::Vector(&U16, None, 64);
static I16x128: Type = Type::Vector(&I16, None, 128);
static U16x128: Type = Type::Vector(&U16, None, 128);

static I32x2: Type = Type::Vector(&I32, None, 2);
static U32x2: Type = Type::Vector(&U32, None, 2);
static I32x4: Type = Type::Vector(&I32, None, 4);
static U32x4: Type = Type::Vector(&U32, None, 4);
static I32x8: Type = Type::Vector(&I32, None, 8);
static U32x8: Type = Type::Vector(&U32, None, 8);
static I32x16: Type = Type::Vector(&I32, None, 16);
static U32x16: Type = Type::Vector(&U32, None, 16);
static I32x32: Type = Type::Vector(&I32, None, 32);
static U32x32: Type = Type::Vector(&U32, None, 32);
static I32x64: Type = Type::Vector(&I32, None, 64);
static U32x64: Type = Type::Vector(&U32, None, 64);

static I64x1: Type = Type::Vector(&I64, None, 1);
static U64x1: Type = Type::Vector(&U64, None, 1);
static I64x2: Type = Type::Vector(&I64, None, 2);
static U64x2: Type = Type::Vector(&U64, None, 2);
static I64x4: Type = Type::Vector(&I64, None, 4);
static U64x4: Type = Type::Vector(&U64, None, 4);

static F32x2: Type = Type::Vector(&F32, None, 2);
static F32x4: Type = Type::Vector(&F32, None, 4);
static F32x8: Type = Type::Vector(&F32, None, 8);
static F64x1: Type = Type::Vector(&F64, None, 1);
static F64x2: Type = Type::Vector(&F64, None, 2);
static F64x4: Type = Type::Vector(&F64, None, 4);

static I32x4_F32: Type = Type::Vector(&I32, Some(&F32), 4);
static I32x8_F32: Type = Type::Vector(&I32, Some(&F32), 8);
static I64x2_F64: Type = Type::Vector(&I64, Some(&F64), 2);
static I64x4_F64: Type = Type::Vector(&I64, Some(&F64), 4);

static VOID: Type = Type::Void;

mod x86;
mod arm;
mod aarch64;
mod nvptx;
mod hexagon;
mod powerpc;

impl Intrinsic {
    pub fn find(name: &str) -> Option<Intrinsic> {
        if name.starts_with("x86_") {
            x86::find(name)
        } else if name.starts_with("arm_") {
            arm::find(name)
        } else if name.starts_with("aarch64_") {
            aarch64::find(name)
        } else if name.starts_with("nvptx_") {
            nvptx::find(name)
        } else if name.starts_with("Q6_") {
            hexagon::find(name)
        } else if name.starts_with("powerpc_") {
            powerpc::find(name)
        } else {
            None
        }
    }
}
