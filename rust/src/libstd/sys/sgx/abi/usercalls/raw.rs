#![allow(unused)]

#[unstable(feature = "sgx_platform", issue = "56975")]
pub use fortanix_sgx_abi::*;

use ptr::NonNull;

#[repr(C)]
struct UsercallReturn(u64, u64);

extern "C" {
    fn usercall(nr: u64, p1: u64, p2: u64, _ignore: u64, p3: u64, p4: u64) -> UsercallReturn;
}

/// Perform the raw usercall operation as defined in the ABI calling convention.
///
/// # Safety
/// The caller must ensure to pass parameters appropriate for the usercall `nr`
/// and to observe all requirements specified in the ABI.
///
/// # Panics
/// Panics if `nr` is 0.
#[unstable(feature = "sgx_platform", issue = "56975")]
pub unsafe fn do_usercall(nr: u64, p1: u64, p2: u64, p3: u64, p4: u64) -> (u64, u64) {
    if nr==0 { panic!("Invalid usercall number {}",nr) }
    let UsercallReturn(a, b) = usercall(nr,p1,p2,0,p3,p4);
    (a, b)
}

type Register = u64;

trait RegisterArgument {
    fn from_register(Register) -> Self;
    fn into_register(self) -> Register;
}

trait ReturnValue {
    fn from_registers(call: &'static str, regs: (Register, Register)) -> Self;
}

macro_rules! define_usercalls {
    // Using `$r:tt` because `$r:ty` doesn't match ! in `clobber_diverging`
    ($(fn $f:ident($($n:ident: $t:ty),*) $(-> $r:tt)*; )*) => {
        #[repr(C)]
        #[allow(non_camel_case_types)]
        enum Usercalls {
            __enclave_usercalls_invalid,
            $($f,)*
        }

        $(enclave_usercalls_internal_define_usercalls!(def fn $f($($n: $t),*) $(-> $r)*);)*
    };
}

macro_rules! define_usercalls_asm {
    ($(fn $f:ident($($n:ident: $t:ty),*) $(-> $r:ty)*; )*) => {
        macro_rules! usercalls_asm {
            () => {
                concat!(
                    ".equ usercall_nr_LAST, 0\n",
                    $(
                    ".equ usercall_nr_", stringify!($f), ", usercall_nr_LAST+1\n",
                    ".equ usercall_nr_LAST, usercall_nr_", stringify!($f), "\n"
                    ),*
                )
            }
        }
    };
}

macro_rules! define_ra {
    (< $i:ident > $t:ty) => {
        impl<$i> RegisterArgument for $t {
            fn from_register(a: Register) -> Self {
                a as _
            }
            fn into_register(self) -> Register {
                self as _
            }
        }
    };
    ($i:ty as $t:ty) => {
        impl RegisterArgument for $t {
            fn from_register(a: Register) -> Self {
                a as $i as _
            }
            fn into_register(self) -> Register {
                self as $i as _
            }
        }
    };
    ($t:ty) => {
        impl RegisterArgument for $t {
            fn from_register(a: Register) -> Self {
                a as _
            }
            fn into_register(self) -> Register {
                self as _
            }
        }
    };
}

define_ra!(Register);
define_ra!(i64);
define_ra!(u32);
define_ra!(u32 as i32);
define_ra!(u16);
define_ra!(u16 as i16);
define_ra!(u8);
define_ra!(u8 as i8);
define_ra!(usize);
define_ra!(usize as isize);
define_ra!(<T> *const T);
define_ra!(<T> *mut T);

impl RegisterArgument for bool {
    fn from_register(a: Register) -> bool {
        if a != 0 {
            true
        } else {
            false
        }
    }
    fn into_register(self) -> Register {
        self as _
    }
}

impl<T: RegisterArgument> RegisterArgument for Option<NonNull<T>> {
    fn from_register(a: Register) -> Option<NonNull<T>> {
        NonNull::new(a as _)
    }
    fn into_register(self) -> Register {
        self.map_or(0 as _, NonNull::as_ptr) as _
    }
}

impl ReturnValue for ! {
    fn from_registers(call: &'static str, _regs: (Register, Register)) -> Self {
        panic!("Usercall {}: did not expect to be re-entered", call);
    }
}

impl ReturnValue for () {
    fn from_registers(call: &'static str, regs: (Register, Register)) -> Self {
        assert_eq!(regs.0, 0, "Usercall {}: expected {} return value to be 0", call, "1st");
        assert_eq!(regs.1, 0, "Usercall {}: expected {} return value to be 0", call, "2nd");
        ()
    }
}

impl<T: RegisterArgument> ReturnValue for T {
    fn from_registers(call: &'static str, regs: (Register, Register)) -> Self {
        assert_eq!(regs.1, 0, "Usercall {}: expected {} return value to be 0", call, "2nd");
        T::from_register(regs.0)
    }
}

impl<T: RegisterArgument, U: RegisterArgument> ReturnValue for (T, U) {
    fn from_registers(_call: &'static str, regs: (Register, Register)) -> Self {
        (
            T::from_register(regs.0),
            U::from_register(regs.1)
        )
    }
}

macro_rules! enclave_usercalls_internal_define_usercalls {
    (def fn $f:ident($n1:ident: $t1:ty, $n2:ident: $t2:ty,
                     $n3:ident: $t3:ty, $n4:ident: $t4:ty) -> $r:ty) => (
        /// This is the raw function definition, see the ABI documentation for
        /// more information.
        #[unstable(feature = "sgx_platform", issue = "56975")]
        #[inline(always)]
        pub unsafe fn $f($n1: $t1, $n2: $t2, $n3: $t3, $n4: $t4) -> $r {
            ReturnValue::from_registers(stringify!($f), do_usercall(
                Usercalls::$f as Register,
                RegisterArgument::into_register($n1),
                RegisterArgument::into_register($n2),
                RegisterArgument::into_register($n3),
                RegisterArgument::into_register($n4),
            ))
        }
    );
    (def fn $f:ident($n1:ident: $t1:ty, $n2:ident: $t2:ty, $n3:ident: $t3:ty) -> $r:ty) => (
        /// This is the raw function definition, see the ABI documentation for
        /// more information.
        #[unstable(feature = "sgx_platform", issue = "56975")]
        #[inline(always)]
        pub unsafe fn $f($n1: $t1, $n2: $t2, $n3: $t3) -> $r {
            ReturnValue::from_registers(stringify!($f), do_usercall(
                Usercalls::$f as Register,
                RegisterArgument::into_register($n1),
                RegisterArgument::into_register($n2),
                RegisterArgument::into_register($n3),
                0
            ))
        }
    );
    (def fn $f:ident($n1:ident: $t1:ty, $n2:ident: $t2:ty) -> $r:ty) => (
        /// This is the raw function definition, see the ABI documentation for
        /// more information.
        #[unstable(feature = "sgx_platform", issue = "56975")]
        #[inline(always)]
        pub unsafe fn $f($n1: $t1, $n2: $t2) -> $r {
            ReturnValue::from_registers(stringify!($f), do_usercall(
                Usercalls::$f as Register,
                RegisterArgument::into_register($n1),
                RegisterArgument::into_register($n2),
                0,0
            ))
        }
    );
    (def fn $f:ident($n1:ident: $t1:ty) -> $r:ty) => (
        /// This is the raw function definition, see the ABI documentation for
        /// more information.
        #[unstable(feature = "sgx_platform", issue = "56975")]
        #[inline(always)]
        pub unsafe fn $f($n1: $t1) -> $r {
            ReturnValue::from_registers(stringify!($f), do_usercall(
                Usercalls::$f as Register,
                RegisterArgument::into_register($n1),
                0,0,0
            ))
        }
    );
    (def fn $f:ident() -> $r:ty) => (
        /// This is the raw function definition, see the ABI documentation for
        /// more information.
        #[unstable(feature = "sgx_platform", issue = "56975")]
        #[inline(always)]
        pub unsafe fn $f() -> $r {
            ReturnValue::from_registers(stringify!($f), do_usercall(
                Usercalls::$f as Register,
                0,0,0,0
            ))
        }
    );
    (def fn $f:ident($($n:ident: $t:ty),*)) => (
        enclave_usercalls_internal_define_usercalls!(def fn $f($($n: $t),*) -> ());
    );
}

invoke_with_usercalls!(define_usercalls);
invoke_with_usercalls!(define_usercalls_asm);
