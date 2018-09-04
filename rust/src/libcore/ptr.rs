// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// FIXME: talk about offset, copy_memory, copy_nonoverlapping_memory

//! Raw, unsafe pointers, `*const T`, and `*mut T`.
//!
//! *[See also the pointer primitive types](../../std/primitive.pointer.html).*

#![stable(feature = "rust1", since = "1.0.0")]

use convert::From;
use intrinsics;
use ops::CoerceUnsized;
use fmt;
use hash;
use marker::{PhantomData, Unsize};
use mem;
use nonzero::NonZero;

use cmp::Ordering::{self, Less, Equal, Greater};

#[stable(feature = "rust1", since = "1.0.0")]
pub use intrinsics::copy_nonoverlapping;

#[stable(feature = "rust1", since = "1.0.0")]
pub use intrinsics::copy;

#[stable(feature = "rust1", since = "1.0.0")]
pub use intrinsics::write_bytes;

/// Executes the destructor (if any) of the pointed-to value.
///
/// This has two use cases:
///
/// * It is *required* to use `drop_in_place` to drop unsized types like
///   trait objects, because they can't be read out onto the stack and
///   dropped normally.
///
/// * It is friendlier to the optimizer to do this over `ptr::read` when
///   dropping manually allocated memory (e.g. when writing Box/Rc/Vec),
///   as the compiler doesn't need to prove that it's sound to elide the
///   copy.
///
/// # Safety
///
/// This has all the same safety problems as `ptr::read` with respect to
/// invalid pointers, types, and double drops.
#[stable(feature = "drop_in_place", since = "1.8.0")]
#[lang = "drop_in_place"]
#[allow(unconditional_recursion)]
pub unsafe fn drop_in_place<T: ?Sized>(to_drop: *mut T) {
    // Code here does not matter - this is replaced by the
    // real drop glue by the compiler.
    drop_in_place(to_drop);
}

/// Creates a null raw pointer.
///
/// # Examples
///
/// ```
/// use std::ptr;
///
/// let p: *const i32 = ptr::null();
/// assert!(p.is_null());
/// ```
#[inline]
#[stable(feature = "rust1", since = "1.0.0")]
pub const fn null<T>() -> *const T { 0 as *const T }

/// Creates a null mutable raw pointer.
///
/// # Examples
///
/// ```
/// use std::ptr;
///
/// let p: *mut i32 = ptr::null_mut();
/// assert!(p.is_null());
/// ```
#[inline]
#[stable(feature = "rust1", since = "1.0.0")]
pub const fn null_mut<T>() -> *mut T { 0 as *mut T }

/// Swaps the values at two mutable locations of the same type, without
/// deinitializing either.
///
/// The values pointed at by `x` and `y` may overlap, unlike `mem::swap` which
/// is otherwise equivalent. If the values do overlap, then the overlapping
/// region of memory from `x` will be used. This is demonstrated in the
/// examples section below.
///
/// # Safety
///
/// This function copies the memory through the raw pointers passed to it
/// as arguments.
///
/// Ensure that these pointers are valid before calling `swap`.
///
/// # Examples
///
/// Swapping two non-overlapping regions:
///
/// ```
/// use std::ptr;
///
/// let mut array = [0, 1, 2, 3];
///
/// let x = array[0..].as_mut_ptr() as *mut [u32; 2];
/// let y = array[2..].as_mut_ptr() as *mut [u32; 2];
///
/// unsafe {
///     ptr::swap(x, y);
///     assert_eq!([2, 3, 0, 1], array);
/// }
/// ```
///
/// Swapping two overlapping regions:
///
/// ```
/// use std::ptr;
///
/// let mut array = [0, 1, 2, 3];
///
/// let x = array[0..].as_mut_ptr() as *mut [u32; 3];
/// let y = array[1..].as_mut_ptr() as *mut [u32; 3];
///
/// unsafe {
///     ptr::swap(x, y);
///     assert_eq!([1, 0, 1, 2], array);
/// }
/// ```
#[inline]
#[stable(feature = "rust1", since = "1.0.0")]
pub unsafe fn swap<T>(x: *mut T, y: *mut T) {
    // Give ourselves some scratch space to work with
    let mut tmp: T = mem::uninitialized();

    // Perform the swap
    copy_nonoverlapping(x, &mut tmp, 1);
    copy(y, x, 1); // `x` and `y` may overlap
    copy_nonoverlapping(&tmp, y, 1);

    // y and t now point to the same thing, but we need to completely forget `tmp`
    // because it's no longer relevant.
    mem::forget(tmp);
}

/// Swaps a sequence of values at two mutable locations of the same type.
///
/// # Safety
///
/// The two arguments must each point to the beginning of `count` locations
/// of valid memory, and the two memory ranges must not overlap.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use std::ptr;
///
/// let mut x = [1, 2, 3, 4];
/// let mut y = [7, 8, 9];
///
/// unsafe {
///     ptr::swap_nonoverlapping(x.as_mut_ptr(), y.as_mut_ptr(), 2);
/// }
///
/// assert_eq!(x, [7, 8, 3, 4]);
/// assert_eq!(y, [1, 2, 9]);
/// ```
#[inline]
#[stable(feature = "swap_nonoverlapping", since = "1.27.0")]
pub unsafe fn swap_nonoverlapping<T>(x: *mut T, y: *mut T, count: usize) {
    let x = x as *mut u8;
    let y = y as *mut u8;
    let len = mem::size_of::<T>() * count;
    swap_nonoverlapping_bytes(x, y, len)
}

#[inline]
unsafe fn swap_nonoverlapping_bytes(x: *mut u8, y: *mut u8, len: usize) {
    // The approach here is to utilize simd to swap x & y efficiently. Testing reveals
    // that swapping either 32 bytes or 64 bytes at a time is most efficient for intel
    // Haswell E processors. LLVM is more able to optimize if we give a struct a
    // #[repr(simd)], even if we don't actually use this struct directly.
    //
    // FIXME repr(simd) broken on emscripten and redox
    // It's also broken on big-endian powerpc64 and s390x.  #42778
    #[cfg_attr(not(any(target_os = "emscripten", target_os = "redox",
                       target_endian = "big")),
               repr(simd))]
    struct Block(u64, u64, u64, u64);
    struct UnalignedBlock(u64, u64, u64, u64);

    let block_size = mem::size_of::<Block>();

    // Loop through x & y, copying them `Block` at a time
    // The optimizer should unroll the loop fully for most types
    // N.B. We can't use a for loop as the `range` impl calls `mem::swap` recursively
    let mut i = 0;
    while i + block_size <= len {
        // Create some uninitialized memory as scratch space
        // Declaring `t` here avoids aligning the stack when this loop is unused
        let mut t: Block = mem::uninitialized();
        let t = &mut t as *mut _ as *mut u8;
        let x = x.offset(i as isize);
        let y = y.offset(i as isize);

        // Swap a block of bytes of x & y, using t as a temporary buffer
        // This should be optimized into efficient SIMD operations where available
        copy_nonoverlapping(x, t, block_size);
        copy_nonoverlapping(y, x, block_size);
        copy_nonoverlapping(t, y, block_size);
        i += block_size;
    }

    if i < len {
        // Swap any remaining bytes
        let mut t: UnalignedBlock = mem::uninitialized();
        let rem = len - i;

        let t = &mut t as *mut _ as *mut u8;
        let x = x.offset(i as isize);
        let y = y.offset(i as isize);

        copy_nonoverlapping(x, t, rem);
        copy_nonoverlapping(y, x, rem);
        copy_nonoverlapping(t, y, rem);
    }
}

/// Moves `src` into the pointed `dest`, returning the previous `dest` value.
///
/// Neither value is dropped.
///
/// # Safety
///
/// This is only unsafe because it accepts a raw pointer.
/// Otherwise, this operation is identical to `mem::replace`.
#[inline]
#[stable(feature = "rust1", since = "1.0.0")]
pub unsafe fn replace<T>(dest: *mut T, mut src: T) -> T {
    mem::swap(&mut *dest, &mut src); // cannot overlap
    src
}

/// Reads the value from `src` without moving it. This leaves the
/// memory in `src` unchanged.
///
/// # Safety
///
/// Beyond accepting a raw pointer, this is unsafe because it semantically
/// moves the value out of `src` without preventing further usage of `src`.
/// If `T` is not `Copy`, then care must be taken to ensure that the value at
/// `src` is not used before the data is overwritten again (e.g. with `write`,
/// `write_bytes`, or `copy`). Note that `*src = foo` counts as a use
/// because it will attempt to drop the value previously at `*src`.
///
/// The pointer must be aligned; use `read_unaligned` if that is not the case.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// let x = 12;
/// let y = &x as *const i32;
///
/// unsafe {
///     assert_eq!(std::ptr::read(y), 12);
/// }
/// ```
#[inline]
#[stable(feature = "rust1", since = "1.0.0")]
pub unsafe fn read<T>(src: *const T) -> T {
    let mut tmp: T = mem::uninitialized();
    copy_nonoverlapping(src, &mut tmp, 1);
    tmp
}

/// Reads the value from `src` without moving it. This leaves the
/// memory in `src` unchanged.
///
/// Unlike `read`, the pointer may be unaligned.
///
/// # Safety
///
/// Beyond accepting a raw pointer, this is unsafe because it semantically
/// moves the value out of `src` without preventing further usage of `src`.
/// If `T` is not `Copy`, then care must be taken to ensure that the value at
/// `src` is not used before the data is overwritten again (e.g. with `write`,
/// `write_bytes`, or `copy`). Note that `*src = foo` counts as a use
/// because it will attempt to drop the value previously at `*src`.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// let x = 12;
/// let y = &x as *const i32;
///
/// unsafe {
///     assert_eq!(std::ptr::read_unaligned(y), 12);
/// }
/// ```
#[inline]
#[stable(feature = "ptr_unaligned", since = "1.17.0")]
pub unsafe fn read_unaligned<T>(src: *const T) -> T {
    let mut tmp: T = mem::uninitialized();
    copy_nonoverlapping(src as *const u8,
                        &mut tmp as *mut T as *mut u8,
                        mem::size_of::<T>());
    tmp
}

/// Overwrites a memory location with the given value without reading or
/// dropping the old value.
///
/// # Safety
///
/// This operation is marked unsafe because it accepts a raw pointer.
///
/// It does not drop the contents of `dst`. This is safe, but it could leak
/// allocations or resources, so care must be taken not to overwrite an object
/// that should be dropped.
///
/// Additionally, it does not drop `src`. Semantically, `src` is moved into the
/// location pointed to by `dst`.
///
/// This is appropriate for initializing uninitialized memory, or overwriting
/// memory that has previously been `read` from.
///
/// The pointer must be aligned; use `write_unaligned` if that is not the case.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// let mut x = 0;
/// let y = &mut x as *mut i32;
/// let z = 12;
///
/// unsafe {
///     std::ptr::write(y, z);
///     assert_eq!(std::ptr::read(y), 12);
/// }
/// ```
#[inline]
#[stable(feature = "rust1", since = "1.0.0")]
pub unsafe fn write<T>(dst: *mut T, src: T) {
    intrinsics::move_val_init(&mut *dst, src)
}

/// Overwrites a memory location with the given value without reading or
/// dropping the old value.
///
/// Unlike `write`, the pointer may be unaligned.
///
/// # Safety
///
/// This operation is marked unsafe because it accepts a raw pointer.
///
/// It does not drop the contents of `dst`. This is safe, but it could leak
/// allocations or resources, so care must be taken not to overwrite an object
/// that should be dropped.
///
/// Additionally, it does not drop `src`. Semantically, `src` is moved into the
/// location pointed to by `dst`.
///
/// This is appropriate for initializing uninitialized memory, or overwriting
/// memory that has previously been `read` from.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// let mut x = 0;
/// let y = &mut x as *mut i32;
/// let z = 12;
///
/// unsafe {
///     std::ptr::write_unaligned(y, z);
///     assert_eq!(std::ptr::read_unaligned(y), 12);
/// }
/// ```
#[inline]
#[stable(feature = "ptr_unaligned", since = "1.17.0")]
pub unsafe fn write_unaligned<T>(dst: *mut T, src: T) {
    copy_nonoverlapping(&src as *const T as *const u8,
                        dst as *mut u8,
                        mem::size_of::<T>());
    mem::forget(src);
}

/// Performs a volatile read of the value from `src` without moving it. This
/// leaves the memory in `src` unchanged.
///
/// Volatile operations are intended to act on I/O memory, and are guaranteed
/// to not be elided or reordered by the compiler across other volatile
/// operations.
///
/// # Notes
///
/// Rust does not currently have a rigorously and formally defined memory model,
/// so the precise semantics of what "volatile" means here is subject to change
/// over time. That being said, the semantics will almost always end up pretty
/// similar to [C11's definition of volatile][c11].
///
/// The compiler shouldn't change the relative order or number of volatile
/// memory operations. However, volatile memory operations on zero-sized types
/// (e.g. if a zero-sized type is passed to `read_volatile`) are no-ops
/// and may be ignored.
///
/// [c11]: http://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf
///
/// # Safety
///
/// Beyond accepting a raw pointer, this is unsafe because it semantically
/// moves the value out of `src` without preventing further usage of `src`.
/// If `T` is not `Copy`, then care must be taken to ensure that the value at
/// `src` is not used before the data is overwritten again (e.g. with `write`,
/// `write_bytes`, or `copy`). Note that `*src = foo` counts as a use
/// because it will attempt to drop the value previously at `*src`.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// let x = 12;
/// let y = &x as *const i32;
///
/// unsafe {
///     assert_eq!(std::ptr::read_volatile(y), 12);
/// }
/// ```
#[inline]
#[stable(feature = "volatile", since = "1.9.0")]
pub unsafe fn read_volatile<T>(src: *const T) -> T {
    intrinsics::volatile_load(src)
}

/// Performs a volatile write of a memory location with the given value without
/// reading or dropping the old value.
///
/// Volatile operations are intended to act on I/O memory, and are guaranteed
/// to not be elided or reordered by the compiler across other volatile
/// operations.
///
/// # Notes
///
/// Rust does not currently have a rigorously and formally defined memory model,
/// so the precise semantics of what "volatile" means here is subject to change
/// over time. That being said, the semantics will almost always end up pretty
/// similar to [C11's definition of volatile][c11].
///
/// The compiler shouldn't change the relative order or number of volatile
/// memory operations. However, volatile memory operations on zero-sized types
/// (e.g. if a zero-sized type is passed to `write_volatile`) are no-ops
/// and may be ignored.
///
/// [c11]: http://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf
///
/// # Safety
///
/// This operation is marked unsafe because it accepts a raw pointer.
///
/// It does not drop the contents of `dst`. This is safe, but it could leak
/// allocations or resources, so care must be taken not to overwrite an object
/// that should be dropped.
///
/// This is appropriate for initializing uninitialized memory, or overwriting
/// memory that has previously been `read` from.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// let mut x = 0;
/// let y = &mut x as *mut i32;
/// let z = 12;
///
/// unsafe {
///     std::ptr::write_volatile(y, z);
///     assert_eq!(std::ptr::read_volatile(y), 12);
/// }
/// ```
#[inline]
#[stable(feature = "volatile", since = "1.9.0")]
pub unsafe fn write_volatile<T>(dst: *mut T, src: T) {
    intrinsics::volatile_store(dst, src);
}

#[lang = "const_ptr"]
impl<T: ?Sized> *const T {
    /// Returns `true` if the pointer is null.
    ///
    /// Note that unsized types have many possible null pointers, as only the
    /// raw data pointer is considered, not their length, vtable, etc.
    /// Therefore, two pointers that are null may still not compare equal to
    /// each other.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let s: &str = "Follow the rabbit";
    /// let ptr: *const u8 = s.as_ptr();
    /// assert!(!ptr.is_null());
    /// ```
    #[stable(feature = "rust1", since = "1.0.0")]
    #[inline]
    pub fn is_null(self) -> bool {
        // Compare via a cast to a thin pointer, so fat pointers are only
        // considering their "data" part for null-ness.
        (self as *const u8) == null()
    }

    /// Returns `None` if the pointer is null, or else returns a reference to
    /// the value wrapped in `Some`.
    ///
    /// # Safety
    ///
    /// While this method and its mutable counterpart are useful for
    /// null-safety, it is important to note that this is still an unsafe
    /// operation because the returned value could be pointing to invalid
    /// memory.
    ///
    /// Additionally, the lifetime `'a` returned is arbitrarily chosen and does
    /// not necessarily reflect the actual lifetime of the data.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let ptr: *const u8 = &10u8 as *const u8;
    ///
    /// unsafe {
    ///     if let Some(val_back) = ptr.as_ref() {
    ///         println!("We got back the value: {}!", val_back);
    ///     }
    /// }
    /// ```
    #[stable(feature = "ptr_as_ref", since = "1.9.0")]
    #[inline]
    pub unsafe fn as_ref<'a>(self) -> Option<&'a T> {
        if self.is_null() {
            None
        } else {
            Some(&*self)
        }
    }

    /// Calculates the offset from a pointer.
    ///
    /// `count` is in units of T; e.g. a `count` of 3 represents a pointer
    /// offset of `3 * size_of::<T>()` bytes.
    ///
    /// # Safety
    ///
    /// If any of the following conditions are violated, the result is Undefined
    /// Behavior:
    ///
    /// * Both the starting and resulting pointer must be either in bounds or one
    ///   byte past the end of an allocated object.
    ///
    /// * The computed offset, **in bytes**, cannot overflow an `isize`.
    ///
    /// * The offset being in bounds cannot rely on "wrapping around" the address
    ///   space. That is, the infinite-precision sum, **in bytes** must fit in a usize.
    ///
    /// The compiler and standard library generally tries to ensure allocations
    /// never reach a size where an offset is a concern. For instance, `Vec`
    /// and `Box` ensure they never allocate more than `isize::MAX` bytes, so
    /// `vec.as_ptr().offset(vec.len() as isize)` is always safe.
    ///
    /// Most platforms fundamentally can't even construct such an allocation.
    /// For instance, no known 64-bit platform can ever serve a request
    /// for 2<sup>63</sup> bytes due to page-table limitations or splitting the address space.
    /// However, some 32-bit and 16-bit platforms may successfully serve a request for
    /// more than `isize::MAX` bytes with things like Physical Address
    /// Extension. As such, memory acquired directly from allocators or memory
    /// mapped files *may* be too large to handle with this function.
    ///
    /// Consider using `wrapping_offset` instead if these constraints are
    /// difficult to satisfy. The only advantage of this method is that it
    /// enables more aggressive compiler optimizations.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let s: &str = "123";
    /// let ptr: *const u8 = s.as_ptr();
    ///
    /// unsafe {
    ///     println!("{}", *ptr.offset(1) as char);
    ///     println!("{}", *ptr.offset(2) as char);
    /// }
    /// ```
    #[stable(feature = "rust1", since = "1.0.0")]
    #[inline]
    pub unsafe fn offset(self, count: isize) -> *const T where T: Sized {
        intrinsics::offset(self, count)
    }

    /// Calculates the offset from a pointer using wrapping arithmetic.
    ///
    /// `count` is in units of T; e.g. a `count` of 3 represents a pointer
    /// offset of `3 * size_of::<T>()` bytes.
    ///
    /// # Safety
    ///
    /// The resulting pointer does not need to be in bounds, but it is
    /// potentially hazardous to dereference (which requires `unsafe`).
    ///
    /// Always use `.offset(count)` instead when possible, because `offset`
    /// allows the compiler to optimize better.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// // Iterate using a raw pointer in increments of two elements
    /// let data = [1u8, 2, 3, 4, 5];
    /// let mut ptr: *const u8 = data.as_ptr();
    /// let step = 2;
    /// let end_rounded_up = ptr.wrapping_offset(6);
    ///
    /// // This loop prints "1, 3, 5, "
    /// while ptr != end_rounded_up {
    ///     unsafe {
    ///         print!("{}, ", *ptr);
    ///     }
    ///     ptr = ptr.wrapping_offset(step);
    /// }
    /// ```
    #[stable(feature = "ptr_wrapping_offset", since = "1.16.0")]
    #[inline]
    pub fn wrapping_offset(self, count: isize) -> *const T where T: Sized {
        unsafe {
            intrinsics::arith_offset(self, count)
        }
    }

    /// Calculates the distance between two pointers. The returned value is in
    /// units of T: the distance in bytes is divided by `mem::size_of::<T>()`.
    ///
    /// If the address different between the two pointers ia not a multiple of
    /// `mem::size_of::<T>()` then the result of the division is rounded towards
    /// zero.
    ///
    /// This function returns `None` if `T` is a zero-sized type.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// #![feature(offset_to)]
    /// #![allow(deprecated)]
    ///
    /// fn main() {
    ///     let a = [0; 5];
    ///     let ptr1: *const i32 = &a[1];
    ///     let ptr2: *const i32 = &a[3];
    ///     assert_eq!(ptr1.offset_to(ptr2), Some(2));
    ///     assert_eq!(ptr2.offset_to(ptr1), Some(-2));
    ///     assert_eq!(unsafe { ptr1.offset(2) }, ptr2);
    ///     assert_eq!(unsafe { ptr2.offset(-2) }, ptr1);
    /// }
    /// ```
    #[unstable(feature = "offset_to", issue = "41079")]
    #[rustc_deprecated(since = "1.27.0", reason = "Replaced by `wrapping_offset_from`, with the \
        opposite argument order.  If you're writing unsafe code, consider `offset_from`.")]
    #[inline]
    pub fn offset_to(self, other: *const T) -> Option<isize> where T: Sized {
        let size = mem::size_of::<T>();
        if size == 0 {
            None
        } else {
            Some(other.wrapping_offset_from(self))
        }
    }

    /// Calculates the distance between two pointers. The returned value is in
    /// units of T: the distance in bytes is divided by `mem::size_of::<T>()`.
    ///
    /// This function is the inverse of [`offset`].
    ///
    /// [`offset`]: #method.offset
    /// [`wrapping_offset_from`]: #method.wrapping_offset_from
    ///
    /// # Safety
    ///
    /// If any of the following conditions are violated, the result is Undefined
    /// Behavior:
    ///
    /// * Both the starting and other pointer must be either in bounds or one
    ///   byte past the end of the same allocated object.
    ///
    /// * The distance between the pointers, **in bytes**, cannot overflow an `isize`.
    ///
    /// * The distance between the pointers, in bytes, must be an exact multiple
    ///   of the size of `T`.
    ///
    /// * The distance being in bounds cannot rely on "wrapping around" the address space.
    ///
    /// The compiler and standard library generally try to ensure allocations
    /// never reach a size where an offset is a concern. For instance, `Vec`
    /// and `Box` ensure they never allocate more than `isize::MAX` bytes, so
    /// `ptr_into_vec.offset_from(vec.as_ptr())` is always safe.
    ///
    /// Most platforms fundamentally can't even construct such an allocation.
    /// For instance, no known 64-bit platform can ever serve a request
    /// for 2<sup>63</sup> bytes due to page-table limitations or splitting the address space.
    /// However, some 32-bit and 16-bit platforms may successfully serve a request for
    /// more than `isize::MAX` bytes with things like Physical Address
    /// Extension. As such, memory acquired directly from allocators or memory
    /// mapped files *may* be too large to handle with this function.
    ///
    /// Consider using [`wrapping_offset_from`] instead if these constraints are
    /// difficult to satisfy. The only advantage of this method is that it
    /// enables more aggressive compiler optimizations.
    ///
    /// # Panics
    ///
    /// This function panics if `T` is a Zero-Sized Type ("ZST").
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// #![feature(ptr_offset_from)]
    ///
    /// let a = [0; 5];
    /// let ptr1: *const i32 = &a[1];
    /// let ptr2: *const i32 = &a[3];
    /// unsafe {
    ///     assert_eq!(ptr2.offset_from(ptr1), 2);
    ///     assert_eq!(ptr1.offset_from(ptr2), -2);
    ///     assert_eq!(ptr1.offset(2), ptr2);
    ///     assert_eq!(ptr2.offset(-2), ptr1);
    /// }
    /// ```
    #[unstable(feature = "ptr_offset_from", issue = "41079")]
    #[inline]
    pub unsafe fn offset_from(self, origin: *const T) -> isize where T: Sized {
        let pointee_size = mem::size_of::<T>();
        assert!(0 < pointee_size && pointee_size <= isize::max_value() as usize);

        // This is the same sequence that Clang emits for pointer subtraction.
        // It can be neither `nsw` nor `nuw` because the input is treated as
        // unsigned but then the output is treated as signed, so neither works.
        let d = isize::wrapping_sub(self as _, origin as _);
        intrinsics::exact_div(d, pointee_size as _)
    }

    /// Calculates the distance between two pointers. The returned value is in
    /// units of T: the distance in bytes is divided by `mem::size_of::<T>()`.
    ///
    /// If the address different between the two pointers is not a multiple of
    /// `mem::size_of::<T>()` then the result of the division is rounded towards
    /// zero.
    ///
    /// Though this method is safe for any two pointers, note that its result
    /// will be mostly useless if the two pointers aren't into the same allocated
    /// object, for example if they point to two different local variables.
    ///
    /// # Panics
    ///
    /// This function panics if `T` is a zero-sized type.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// #![feature(ptr_wrapping_offset_from)]
    ///
    /// let a = [0; 5];
    /// let ptr1: *const i32 = &a[1];
    /// let ptr2: *const i32 = &a[3];
    /// assert_eq!(ptr2.wrapping_offset_from(ptr1), 2);
    /// assert_eq!(ptr1.wrapping_offset_from(ptr2), -2);
    /// assert_eq!(ptr1.wrapping_offset(2), ptr2);
    /// assert_eq!(ptr2.wrapping_offset(-2), ptr1);
    ///
    /// let ptr1: *const i32 = 3 as _;
    /// let ptr2: *const i32 = 13 as _;
    /// assert_eq!(ptr2.wrapping_offset_from(ptr1), 2);
    /// ```
    #[unstable(feature = "ptr_wrapping_offset_from", issue = "41079")]
    #[inline]
    pub fn wrapping_offset_from(self, origin: *const T) -> isize where T: Sized {
        let pointee_size = mem::size_of::<T>();
        assert!(0 < pointee_size && pointee_size <= isize::max_value() as usize);

        let d = isize::wrapping_sub(self as _, origin as _);
        d.wrapping_div(pointee_size as _)
    }

    /// Calculates the offset from a pointer (convenience for `.offset(count as isize)`).
    ///
    /// `count` is in units of T; e.g. a `count` of 3 represents a pointer
    /// offset of `3 * size_of::<T>()` bytes.
    ///
    /// # Safety
    ///
    /// If any of the following conditions are violated, the result is Undefined
    /// Behavior:
    ///
    /// * Both the starting and resulting pointer must be either in bounds or one
    ///   byte past the end of an allocated object.
    ///
    /// * The computed offset, **in bytes**, cannot overflow an `isize`.
    ///
    /// * The offset being in bounds cannot rely on "wrapping around" the address
    ///   space. That is, the infinite-precision sum must fit in a `usize`.
    ///
    /// The compiler and standard library generally tries to ensure allocations
    /// never reach a size where an offset is a concern. For instance, `Vec`
    /// and `Box` ensure they never allocate more than `isize::MAX` bytes, so
    /// `vec.as_ptr().add(vec.len())` is always safe.
    ///
    /// Most platforms fundamentally can't even construct such an allocation.
    /// For instance, no known 64-bit platform can ever serve a request
    /// for 2<sup>63</sup> bytes due to page-table limitations or splitting the address space.
    /// However, some 32-bit and 16-bit platforms may successfully serve a request for
    /// more than `isize::MAX` bytes with things like Physical Address
    /// Extension. As such, memory acquired directly from allocators or memory
    /// mapped files *may* be too large to handle with this function.
    ///
    /// Consider using `wrapping_offset` instead if these constraints are
    /// difficult to satisfy. The only advantage of this method is that it
    /// enables more aggressive compiler optimizations.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let s: &str = "123";
    /// let ptr: *const u8 = s.as_ptr();
    ///
    /// unsafe {
    ///     println!("{}", *ptr.add(1) as char);
    ///     println!("{}", *ptr.add(2) as char);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn add(self, count: usize) -> Self
        where T: Sized,
    {
        self.offset(count as isize)
    }

    /// Calculates the offset from a pointer (convenience for
    /// `.offset((count as isize).wrapping_neg())`).
    ///
    /// `count` is in units of T; e.g. a `count` of 3 represents a pointer
    /// offset of `3 * size_of::<T>()` bytes.
    ///
    /// # Safety
    ///
    /// If any of the following conditions are violated, the result is Undefined
    /// Behavior:
    ///
    /// * Both the starting and resulting pointer must be either in bounds or one
    ///   byte past the end of an allocated object.
    ///
    /// * The computed offset cannot exceed `isize::MAX` **bytes**.
    ///
    /// * The offset being in bounds cannot rely on "wrapping around" the address
    ///   space. That is, the infinite-precision sum must fit in a usize.
    ///
    /// The compiler and standard library generally tries to ensure allocations
    /// never reach a size where an offset is a concern. For instance, `Vec`
    /// and `Box` ensure they never allocate more than `isize::MAX` bytes, so
    /// `vec.as_ptr().add(vec.len()).sub(vec.len())` is always safe.
    ///
    /// Most platforms fundamentally can't even construct such an allocation.
    /// For instance, no known 64-bit platform can ever serve a request
    /// for 2<sup>63</sup> bytes due to page-table limitations or splitting the address space.
    /// However, some 32-bit and 16-bit platforms may successfully serve a request for
    /// more than `isize::MAX` bytes with things like Physical Address
    /// Extension. As such, memory acquired directly from allocators or memory
    /// mapped files *may* be too large to handle with this function.
    ///
    /// Consider using `wrapping_offset` instead if these constraints are
    /// difficult to satisfy. The only advantage of this method is that it
    /// enables more aggressive compiler optimizations.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let s: &str = "123";
    ///
    /// unsafe {
    ///     let end: *const u8 = s.as_ptr().add(3);
    ///     println!("{}", *end.sub(1) as char);
    ///     println!("{}", *end.sub(2) as char);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn sub(self, count: usize) -> Self
        where T: Sized,
    {
        self.offset((count as isize).wrapping_neg())
    }

    /// Calculates the offset from a pointer using wrapping arithmetic.
    /// (convenience for `.wrapping_offset(count as isize)`)
    ///
    /// `count` is in units of T; e.g. a `count` of 3 represents a pointer
    /// offset of `3 * size_of::<T>()` bytes.
    ///
    /// # Safety
    ///
    /// The resulting pointer does not need to be in bounds, but it is
    /// potentially hazardous to dereference (which requires `unsafe`).
    ///
    /// Always use `.add(count)` instead when possible, because `add`
    /// allows the compiler to optimize better.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// // Iterate using a raw pointer in increments of two elements
    /// let data = [1u8, 2, 3, 4, 5];
    /// let mut ptr: *const u8 = data.as_ptr();
    /// let step = 2;
    /// let end_rounded_up = ptr.wrapping_add(6);
    ///
    /// // This loop prints "1, 3, 5, "
    /// while ptr != end_rounded_up {
    ///     unsafe {
    ///         print!("{}, ", *ptr);
    ///     }
    ///     ptr = ptr.wrapping_add(step);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub fn wrapping_add(self, count: usize) -> Self
        where T: Sized,
    {
        self.wrapping_offset(count as isize)
    }

    /// Calculates the offset from a pointer using wrapping arithmetic.
    /// (convenience for `.wrapping_offset((count as isize).wrapping_sub())`)
    ///
    /// `count` is in units of T; e.g. a `count` of 3 represents a pointer
    /// offset of `3 * size_of::<T>()` bytes.
    ///
    /// # Safety
    ///
    /// The resulting pointer does not need to be in bounds, but it is
    /// potentially hazardous to dereference (which requires `unsafe`).
    ///
    /// Always use `.sub(count)` instead when possible, because `sub`
    /// allows the compiler to optimize better.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// // Iterate using a raw pointer in increments of two elements (backwards)
    /// let data = [1u8, 2, 3, 4, 5];
    /// let mut ptr: *const u8 = data.as_ptr();
    /// let start_rounded_down = ptr.wrapping_sub(2);
    /// ptr = ptr.wrapping_add(4);
    /// let step = 2;
    /// // This loop prints "5, 3, 1, "
    /// while ptr != start_rounded_down {
    ///     unsafe {
    ///         print!("{}, ", *ptr);
    ///     }
    ///     ptr = ptr.wrapping_sub(step);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub fn wrapping_sub(self, count: usize) -> Self
        where T: Sized,
    {
        self.wrapping_offset((count as isize).wrapping_neg())
    }

    /// Reads the value from `self` without moving it. This leaves the
    /// memory in `self` unchanged.
    ///
    /// # Safety
    ///
    /// Beyond accepting a raw pointer, this is unsafe because it semantically
    /// moves the value out of `self` without preventing further usage of `self`.
    /// If `T` is not `Copy`, then care must be taken to ensure that the value at
    /// `self` is not used before the data is overwritten again (e.g. with `write`,
    /// `write_bytes`, or `copy`). Note that `*self = foo` counts as a use
    /// because it will attempt to drop the value previously at `*self`.
    ///
    /// The pointer must be aligned; use `read_unaligned` if that is not the case.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let x = 12;
    /// let y = &x as *const i32;
    ///
    /// unsafe {
    ///     assert_eq!(y.read(), 12);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn read(self) -> T
        where T: Sized,
    {
        read(self)
    }

    /// Performs a volatile read of the value from `self` without moving it. This
    /// leaves the memory in `self` unchanged.
    ///
    /// Volatile operations are intended to act on I/O memory, and are guaranteed
    /// to not be elided or reordered by the compiler across other volatile
    /// operations.
    ///
    /// # Notes
    ///
    /// Rust does not currently have a rigorously and formally defined memory model,
    /// so the precise semantics of what "volatile" means here is subject to change
    /// over time. That being said, the semantics will almost always end up pretty
    /// similar to [C11's definition of volatile][c11].
    ///
    /// The compiler shouldn't change the relative order or number of volatile
    /// memory operations. However, volatile memory operations on zero-sized types
    /// (e.g. if a zero-sized type is passed to `read_volatile`) are no-ops
    /// and may be ignored.
    ///
    /// [c11]: http://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf
    ///
    /// # Safety
    ///
    /// Beyond accepting a raw pointer, this is unsafe because it semantically
    /// moves the value out of `self` without preventing further usage of `self`.
    /// If `T` is not `Copy`, then care must be taken to ensure that the value at
    /// `self` is not used before the data is overwritten again (e.g. with `write`,
    /// `write_bytes`, or `copy`). Note that `*self = foo` counts as a use
    /// because it will attempt to drop the value previously at `*self`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let x = 12;
    /// let y = &x as *const i32;
    ///
    /// unsafe {
    ///     assert_eq!(y.read_volatile(), 12);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn read_volatile(self) -> T
        where T: Sized,
    {
        read_volatile(self)
    }

    /// Reads the value from `self` without moving it. This leaves the
    /// memory in `self` unchanged.
    ///
    /// Unlike `read`, the pointer may be unaligned.
    ///
    /// # Safety
    ///
    /// Beyond accepting a raw pointer, this is unsafe because it semantically
    /// moves the value out of `self` without preventing further usage of `self`.
    /// If `T` is not `Copy`, then care must be taken to ensure that the value at
    /// `self` is not used before the data is overwritten again (e.g. with `write`,
    /// `write_bytes`, or `copy`). Note that `*self = foo` counts as a use
    /// because it will attempt to drop the value previously at `*self`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let x = 12;
    /// let y = &x as *const i32;
    ///
    /// unsafe {
    ///     assert_eq!(y.read_unaligned(), 12);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn read_unaligned(self) -> T
        where T: Sized,
    {
        read_unaligned(self)
    }

    /// Copies `count * size_of<T>` bytes from `self` to `dest`. The source
    /// and destination may overlap.
    ///
    /// NOTE: this has the *same* argument order as `ptr::copy`.
    ///
    /// This is semantically equivalent to C's `memmove`.
    ///
    /// # Safety
    ///
    /// Care must be taken with the ownership of `self` and `dest`.
    /// This method semantically moves the values of `self` into `dest`.
    /// However it does not drop the contents of `self`, or prevent the contents
    /// of `dest` from being dropped or used.
    ///
    /// # Examples
    ///
    /// Efficiently create a Rust vector from an unsafe buffer:
    ///
    /// ```
    /// # #[allow(dead_code)]
    /// unsafe fn from_buf_raw<T: Copy>(ptr: *const T, elts: usize) -> Vec<T> {
    ///     let mut dst = Vec::with_capacity(elts);
    ///     dst.set_len(elts);
    ///     ptr.copy_to(dst.as_mut_ptr(), elts);
    ///     dst
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn copy_to(self, dest: *mut T, count: usize)
        where T: Sized,
    {
        copy(self, dest, count)
    }

    /// Copies `count * size_of<T>` bytes from `self` to `dest`. The source
    /// and destination may *not* overlap.
    ///
    /// NOTE: this has the *same* argument order as `ptr::copy_nonoverlapping`.
    ///
    /// `copy_nonoverlapping` is semantically equivalent to C's `memcpy`.
    ///
    /// # Safety
    ///
    /// Beyond requiring that the program must be allowed to access both regions
    /// of memory, it is Undefined Behavior for source and destination to
    /// overlap. Care must also be taken with the ownership of `self` and
    /// `self`. This method semantically moves the values of `self` into `dest`.
    /// However it does not drop the contents of `dest`, or prevent the contents
    /// of `self` from being dropped or used.
    ///
    /// # Examples
    ///
    /// Efficiently create a Rust vector from an unsafe buffer:
    ///
    /// ```
    /// # #[allow(dead_code)]
    /// unsafe fn from_buf_raw<T: Copy>(ptr: *const T, elts: usize) -> Vec<T> {
    ///     let mut dst = Vec::with_capacity(elts);
    ///     dst.set_len(elts);
    ///     ptr.copy_to_nonoverlapping(dst.as_mut_ptr(), elts);
    ///     dst
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn copy_to_nonoverlapping(self, dest: *mut T, count: usize)
        where T: Sized,
    {
        copy_nonoverlapping(self, dest, count)
    }

    /// Computes the offset that needs to be applied to the pointer in order to make it aligned to
    /// `align`.
    ///
    /// If it is not possible to align the pointer, the implementation returns
    /// `usize::max_value()`.
    ///
    /// The offset is expressed in number of `T` elements, and not bytes. The value returned can be
    /// used with the `offset` or `offset_to` methods.
    ///
    /// There are no guarantees whatsover that offsetting the pointer will not overflow or go
    /// beyond the allocation that the pointer points into. It is up to the caller to ensure that
    /// the returned offset is correct in all terms other than alignment.
    ///
    /// # Panics
    ///
    /// The function panics if `align` is not a power-of-two.
    ///
    /// # Examples
    ///
    /// Accessing adjacent `u8` as `u16`
    ///
    /// ```
    /// # #![feature(align_offset)]
    /// # fn foo(n: usize) {
    /// # use std::mem::align_of;
    /// # unsafe {
    /// let x = [5u8, 6u8, 7u8, 8u8, 9u8];
    /// let ptr = &x[n] as *const u8;
    /// let offset = ptr.align_offset(align_of::<u16>());
    /// if offset < x.len() - n - 1 {
    ///     let u16_ptr = ptr.offset(offset as isize) as *const u16;
    ///     assert_ne!(*u16_ptr, 500);
    /// } else {
    ///     // while the pointer can be aligned via `offset`, it would point
    ///     // outside the allocation
    /// }
    /// # } }
    /// ```
    #[unstable(feature = "align_offset", issue = "44488")]
    #[cfg(not(stage0))]
    pub fn align_offset(self, align: usize) -> usize where T: Sized {
        if !align.is_power_of_two() {
            panic!("align_offset: align is not a power-of-two");
        }
        unsafe {
            align_offset(self, align)
        }
    }

    /// definitely docs.
    #[unstable(feature = "align_offset", issue = "44488")]
    #[cfg(stage0)]
    pub fn align_offset(self, align: usize) -> usize where T: Sized {
        if !align.is_power_of_two() {
            panic!("align_offset: align is not a power-of-two");
        }
        unsafe {
            intrinsics::align_offset(self as *const (), align)
        }
    }
}


#[lang = "mut_ptr"]
impl<T: ?Sized> *mut T {
    /// Returns `true` if the pointer is null.
    ///
    /// Note that unsized types have many possible null pointers, as only the
    /// raw data pointer is considered, not their length, vtable, etc.
    /// Therefore, two pointers that are null may still not compare equal to
    /// each other.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let mut s = [1, 2, 3];
    /// let ptr: *mut u32 = s.as_mut_ptr();
    /// assert!(!ptr.is_null());
    /// ```
    #[stable(feature = "rust1", since = "1.0.0")]
    #[inline]
    pub fn is_null(self) -> bool {
        // Compare via a cast to a thin pointer, so fat pointers are only
        // considering their "data" part for null-ness.
        (self as *mut u8) == null_mut()
    }

    /// Returns `None` if the pointer is null, or else returns a reference to
    /// the value wrapped in `Some`.
    ///
    /// # Safety
    ///
    /// While this method and its mutable counterpart are useful for
    /// null-safety, it is important to note that this is still an unsafe
    /// operation because the returned value could be pointing to invalid
    /// memory.
    ///
    /// Additionally, the lifetime `'a` returned is arbitrarily chosen and does
    /// not necessarily reflect the actual lifetime of the data.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let ptr: *mut u8 = &mut 10u8 as *mut u8;
    ///
    /// unsafe {
    ///     if let Some(val_back) = ptr.as_ref() {
    ///         println!("We got back the value: {}!", val_back);
    ///     }
    /// }
    /// ```
    #[stable(feature = "ptr_as_ref", since = "1.9.0")]
    #[inline]
    pub unsafe fn as_ref<'a>(self) -> Option<&'a T> {
        if self.is_null() {
            None
        } else {
            Some(&*self)
        }
    }

    /// Calculates the offset from a pointer.
    ///
    /// `count` is in units of T; e.g. a `count` of 3 represents a pointer
    /// offset of `3 * size_of::<T>()` bytes.
    ///
    /// # Safety
    ///
    /// If any of the following conditions are violated, the result is Undefined
    /// Behavior:
    ///
    /// * Both the starting and resulting pointer must be either in bounds or one
    ///   byte past the end of an allocated object.
    ///
    /// * The computed offset, **in bytes**, cannot overflow an `isize`.
    ///
    /// * The offset being in bounds cannot rely on "wrapping around" the address
    ///   space. That is, the infinite-precision sum, **in bytes** must fit in a usize.
    ///
    /// The compiler and standard library generally tries to ensure allocations
    /// never reach a size where an offset is a concern. For instance, `Vec`
    /// and `Box` ensure they never allocate more than `isize::MAX` bytes, so
    /// `vec.as_ptr().offset(vec.len() as isize)` is always safe.
    ///
    /// Most platforms fundamentally can't even construct such an allocation.
    /// For instance, no known 64-bit platform can ever serve a request
    /// for 2<sup>63</sup> bytes due to page-table limitations or splitting the address space.
    /// However, some 32-bit and 16-bit platforms may successfully serve a request for
    /// more than `isize::MAX` bytes with things like Physical Address
    /// Extension. As such, memory acquired directly from allocators or memory
    /// mapped files *may* be too large to handle with this function.
    ///
    /// Consider using `wrapping_offset` instead if these constraints are
    /// difficult to satisfy. The only advantage of this method is that it
    /// enables more aggressive compiler optimizations.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let mut s = [1, 2, 3];
    /// let ptr: *mut u32 = s.as_mut_ptr();
    ///
    /// unsafe {
    ///     println!("{}", *ptr.offset(1));
    ///     println!("{}", *ptr.offset(2));
    /// }
    /// ```
    #[stable(feature = "rust1", since = "1.0.0")]
    #[inline]
    pub unsafe fn offset(self, count: isize) -> *mut T where T: Sized {
        intrinsics::offset(self, count) as *mut T
    }

    /// Calculates the offset from a pointer using wrapping arithmetic.
    /// `count` is in units of T; e.g. a `count` of 3 represents a pointer
    /// offset of `3 * size_of::<T>()` bytes.
    ///
    /// # Safety
    ///
    /// The resulting pointer does not need to be in bounds, but it is
    /// potentially hazardous to dereference (which requires `unsafe`).
    ///
    /// Always use `.offset(count)` instead when possible, because `offset`
    /// allows the compiler to optimize better.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// // Iterate using a raw pointer in increments of two elements
    /// let mut data = [1u8, 2, 3, 4, 5];
    /// let mut ptr: *mut u8 = data.as_mut_ptr();
    /// let step = 2;
    /// let end_rounded_up = ptr.wrapping_offset(6);
    ///
    /// while ptr != end_rounded_up {
    ///     unsafe {
    ///         *ptr = 0;
    ///     }
    ///     ptr = ptr.wrapping_offset(step);
    /// }
    /// assert_eq!(&data, &[0, 2, 0, 4, 0]);
    /// ```
    #[stable(feature = "ptr_wrapping_offset", since = "1.16.0")]
    #[inline]
    pub fn wrapping_offset(self, count: isize) -> *mut T where T: Sized {
        unsafe {
            intrinsics::arith_offset(self, count) as *mut T
        }
    }

    /// Returns `None` if the pointer is null, or else returns a mutable
    /// reference to the value wrapped in `Some`.
    ///
    /// # Safety
    ///
    /// As with `as_ref`, this is unsafe because it cannot verify the validity
    /// of the returned pointer, nor can it ensure that the lifetime `'a`
    /// returned is indeed a valid lifetime for the contained data.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let mut s = [1, 2, 3];
    /// let ptr: *mut u32 = s.as_mut_ptr();
    /// let first_value = unsafe { ptr.as_mut().unwrap() };
    /// *first_value = 4;
    /// println!("{:?}", s); // It'll print: "[4, 2, 3]".
    /// ```
    #[stable(feature = "ptr_as_ref", since = "1.9.0")]
    #[inline]
    pub unsafe fn as_mut<'a>(self) -> Option<&'a mut T> {
        if self.is_null() {
            None
        } else {
            Some(&mut *self)
        }
    }

    /// Calculates the distance between two pointers. The returned value is in
    /// units of T: the distance in bytes is divided by `mem::size_of::<T>()`.
    ///
    /// If the address different between the two pointers ia not a multiple of
    /// `mem::size_of::<T>()` then the result of the division is rounded towards
    /// zero.
    ///
    /// This function returns `None` if `T` is a zero-sized type.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// #![feature(offset_to)]
    /// #![allow(deprecated)]
    ///
    /// fn main() {
    ///     let mut a = [0; 5];
    ///     let ptr1: *mut i32 = &mut a[1];
    ///     let ptr2: *mut i32 = &mut a[3];
    ///     assert_eq!(ptr1.offset_to(ptr2), Some(2));
    ///     assert_eq!(ptr2.offset_to(ptr1), Some(-2));
    ///     assert_eq!(unsafe { ptr1.offset(2) }, ptr2);
    ///     assert_eq!(unsafe { ptr2.offset(-2) }, ptr1);
    /// }
    /// ```
    #[unstable(feature = "offset_to", issue = "41079")]
    #[rustc_deprecated(since = "1.27.0", reason = "Replaced by `wrapping_offset_from`, with the \
        opposite argument order.  If you're writing unsafe code, consider `offset_from`.")]
    #[inline]
    pub fn offset_to(self, other: *const T) -> Option<isize> where T: Sized {
        let size = mem::size_of::<T>();
        if size == 0 {
            None
        } else {
            Some(other.wrapping_offset_from(self))
        }
    }

    /// Calculates the distance between two pointers. The returned value is in
    /// units of T: the distance in bytes is divided by `mem::size_of::<T>()`.
    ///
    /// This function is the inverse of [`offset`].
    ///
    /// [`offset`]: #method.offset-1
    /// [`wrapping_offset_from`]: #method.wrapping_offset_from-1
    ///
    /// # Safety
    ///
    /// If any of the following conditions are violated, the result is Undefined
    /// Behavior:
    ///
    /// * Both the starting and other pointer must be either in bounds or one
    ///   byte past the end of the same allocated object.
    ///
    /// * The distance between the pointers, **in bytes**, cannot overflow an `isize`.
    ///
    /// * The distance between the pointers, in bytes, must be an exact multiple
    ///   of the size of `T`.
    ///
    /// * The distance being in bounds cannot rely on "wrapping around" the address space.
    ///
    /// The compiler and standard library generally try to ensure allocations
    /// never reach a size where an offset is a concern. For instance, `Vec`
    /// and `Box` ensure they never allocate more than `isize::MAX` bytes, so
    /// `ptr_into_vec.offset_from(vec.as_ptr())` is always safe.
    ///
    /// Most platforms fundamentally can't even construct such an allocation.
    /// For instance, no known 64-bit platform can ever serve a request
    /// for 2<sup>63</sup> bytes due to page-table limitations or splitting the address space.
    /// However, some 32-bit and 16-bit platforms may successfully serve a request for
    /// more than `isize::MAX` bytes with things like Physical Address
    /// Extension. As such, memory acquired directly from allocators or memory
    /// mapped files *may* be too large to handle with this function.
    ///
    /// Consider using [`wrapping_offset_from`] instead if these constraints are
    /// difficult to satisfy. The only advantage of this method is that it
    /// enables more aggressive compiler optimizations.
    ///
    /// # Panics
    ///
    /// This function panics if `T` is a Zero-Sized Type ("ZST").
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// #![feature(ptr_offset_from)]
    ///
    /// let mut a = [0; 5];
    /// let ptr1: *mut i32 = &mut a[1];
    /// let ptr2: *mut i32 = &mut a[3];
    /// unsafe {
    ///     assert_eq!(ptr2.offset_from(ptr1), 2);
    ///     assert_eq!(ptr1.offset_from(ptr2), -2);
    ///     assert_eq!(ptr1.offset(2), ptr2);
    ///     assert_eq!(ptr2.offset(-2), ptr1);
    /// }
    /// ```
    #[unstable(feature = "ptr_offset_from", issue = "41079")]
    #[inline]
    pub unsafe fn offset_from(self, origin: *const T) -> isize where T: Sized {
        (self as *const T).offset_from(origin)
    }

    /// Calculates the distance between two pointers. The returned value is in
    /// units of T: the distance in bytes is divided by `mem::size_of::<T>()`.
    ///
    /// If the address different between the two pointers is not a multiple of
    /// `mem::size_of::<T>()` then the result of the division is rounded towards
    /// zero.
    ///
    /// Though this method is safe for any two pointers, note that its result
    /// will be mostly useless if the two pointers aren't into the same allocated
    /// object, for example if they point to two different local variables.
    ///
    /// # Panics
    ///
    /// This function panics if `T` is a zero-sized type.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// #![feature(ptr_wrapping_offset_from)]
    ///
    /// let mut a = [0; 5];
    /// let ptr1: *mut i32 = &mut a[1];
    /// let ptr2: *mut i32 = &mut a[3];
    /// assert_eq!(ptr2.wrapping_offset_from(ptr1), 2);
    /// assert_eq!(ptr1.wrapping_offset_from(ptr2), -2);
    /// assert_eq!(ptr1.wrapping_offset(2), ptr2);
    /// assert_eq!(ptr2.wrapping_offset(-2), ptr1);
    ///
    /// let ptr1: *mut i32 = 3 as _;
    /// let ptr2: *mut i32 = 13 as _;
    /// assert_eq!(ptr2.wrapping_offset_from(ptr1), 2);
    /// ```
    #[unstable(feature = "ptr_wrapping_offset_from", issue = "41079")]
    #[inline]
    pub fn wrapping_offset_from(self, origin: *const T) -> isize where T: Sized {
        (self as *const T).wrapping_offset_from(origin)
    }

    /// Calculates the offset from a pointer (convenience for `.offset(count as isize)`).
    ///
    /// `count` is in units of T; e.g. a `count` of 3 represents a pointer
    /// offset of `3 * size_of::<T>()` bytes.
    ///
    /// # Safety
    ///
    /// If any of the following conditions are violated, the result is Undefined
    /// Behavior:
    ///
    /// * Both the starting and resulting pointer must be either in bounds or one
    ///   byte past the end of an allocated object.
    ///
    /// * The computed offset, **in bytes**, cannot overflow an `isize`.
    ///
    /// * The offset being in bounds cannot rely on "wrapping around" the address
    ///   space. That is, the infinite-precision sum must fit in a `usize`.
    ///
    /// The compiler and standard library generally tries to ensure allocations
    /// never reach a size where an offset is a concern. For instance, `Vec`
    /// and `Box` ensure they never allocate more than `isize::MAX` bytes, so
    /// `vec.as_ptr().add(vec.len())` is always safe.
    ///
    /// Most platforms fundamentally can't even construct such an allocation.
    /// For instance, no known 64-bit platform can ever serve a request
    /// for 2<sup>63</sup> bytes due to page-table limitations or splitting the address space.
    /// However, some 32-bit and 16-bit platforms may successfully serve a request for
    /// more than `isize::MAX` bytes with things like Physical Address
    /// Extension. As such, memory acquired directly from allocators or memory
    /// mapped files *may* be too large to handle with this function.
    ///
    /// Consider using `wrapping_offset` instead if these constraints are
    /// difficult to satisfy. The only advantage of this method is that it
    /// enables more aggressive compiler optimizations.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let s: &str = "123";
    /// let ptr: *const u8 = s.as_ptr();
    ///
    /// unsafe {
    ///     println!("{}", *ptr.add(1) as char);
    ///     println!("{}", *ptr.add(2) as char);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn add(self, count: usize) -> Self
        where T: Sized,
    {
        self.offset(count as isize)
    }

    /// Calculates the offset from a pointer (convenience for
    /// `.offset((count as isize).wrapping_neg())`).
    ///
    /// `count` is in units of T; e.g. a `count` of 3 represents a pointer
    /// offset of `3 * size_of::<T>()` bytes.
    ///
    /// # Safety
    ///
    /// If any of the following conditions are violated, the result is Undefined
    /// Behavior:
    ///
    /// * Both the starting and resulting pointer must be either in bounds or one
    ///   byte past the end of an allocated object.
    ///
    /// * The computed offset cannot exceed `isize::MAX` **bytes**.
    ///
    /// * The offset being in bounds cannot rely on "wrapping around" the address
    ///   space. That is, the infinite-precision sum must fit in a usize.
    ///
    /// The compiler and standard library generally tries to ensure allocations
    /// never reach a size where an offset is a concern. For instance, `Vec`
    /// and `Box` ensure they never allocate more than `isize::MAX` bytes, so
    /// `vec.as_ptr().add(vec.len()).sub(vec.len())` is always safe.
    ///
    /// Most platforms fundamentally can't even construct such an allocation.
    /// For instance, no known 64-bit platform can ever serve a request
    /// for 2<sup>63</sup> bytes due to page-table limitations or splitting the address space.
    /// However, some 32-bit and 16-bit platforms may successfully serve a request for
    /// more than `isize::MAX` bytes with things like Physical Address
    /// Extension. As such, memory acquired directly from allocators or memory
    /// mapped files *may* be too large to handle with this function.
    ///
    /// Consider using `wrapping_offset` instead if these constraints are
    /// difficult to satisfy. The only advantage of this method is that it
    /// enables more aggressive compiler optimizations.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let s: &str = "123";
    ///
    /// unsafe {
    ///     let end: *const u8 = s.as_ptr().add(3);
    ///     println!("{}", *end.sub(1) as char);
    ///     println!("{}", *end.sub(2) as char);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn sub(self, count: usize) -> Self
        where T: Sized,
    {
        self.offset((count as isize).wrapping_neg())
    }

    /// Calculates the offset from a pointer using wrapping arithmetic.
    /// (convenience for `.wrapping_offset(count as isize)`)
    ///
    /// `count` is in units of T; e.g. a `count` of 3 represents a pointer
    /// offset of `3 * size_of::<T>()` bytes.
    ///
    /// # Safety
    ///
    /// The resulting pointer does not need to be in bounds, but it is
    /// potentially hazardous to dereference (which requires `unsafe`).
    ///
    /// Always use `.add(count)` instead when possible, because `add`
    /// allows the compiler to optimize better.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// // Iterate using a raw pointer in increments of two elements
    /// let data = [1u8, 2, 3, 4, 5];
    /// let mut ptr: *const u8 = data.as_ptr();
    /// let step = 2;
    /// let end_rounded_up = ptr.wrapping_add(6);
    ///
    /// // This loop prints "1, 3, 5, "
    /// while ptr != end_rounded_up {
    ///     unsafe {
    ///         print!("{}, ", *ptr);
    ///     }
    ///     ptr = ptr.wrapping_add(step);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub fn wrapping_add(self, count: usize) -> Self
        where T: Sized,
    {
        self.wrapping_offset(count as isize)
    }

    /// Calculates the offset from a pointer using wrapping arithmetic.
    /// (convenience for `.wrapping_offset((count as isize).wrapping_sub())`)
    ///
    /// `count` is in units of T; e.g. a `count` of 3 represents a pointer
    /// offset of `3 * size_of::<T>()` bytes.
    ///
    /// # Safety
    ///
    /// The resulting pointer does not need to be in bounds, but it is
    /// potentially hazardous to dereference (which requires `unsafe`).
    ///
    /// Always use `.sub(count)` instead when possible, because `sub`
    /// allows the compiler to optimize better.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// // Iterate using a raw pointer in increments of two elements (backwards)
    /// let data = [1u8, 2, 3, 4, 5];
    /// let mut ptr: *const u8 = data.as_ptr();
    /// let start_rounded_down = ptr.wrapping_sub(2);
    /// ptr = ptr.wrapping_add(4);
    /// let step = 2;
    /// // This loop prints "5, 3, 1, "
    /// while ptr != start_rounded_down {
    ///     unsafe {
    ///         print!("{}, ", *ptr);
    ///     }
    ///     ptr = ptr.wrapping_sub(step);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub fn wrapping_sub(self, count: usize) -> Self
        where T: Sized,
    {
        self.wrapping_offset((count as isize).wrapping_neg())
    }

    /// Reads the value from `self` without moving it. This leaves the
    /// memory in `self` unchanged.
    ///
    /// # Safety
    ///
    /// Beyond accepting a raw pointer, this is unsafe because it semantically
    /// moves the value out of `self` without preventing further usage of `self`.
    /// If `T` is not `Copy`, then care must be taken to ensure that the value at
    /// `self` is not used before the data is overwritten again (e.g. with `write`,
    /// `write_bytes`, or `copy`). Note that `*self = foo` counts as a use
    /// because it will attempt to drop the value previously at `*self`.
    ///
    /// The pointer must be aligned; use `read_unaligned` if that is not the case.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let x = 12;
    /// let y = &x as *const i32;
    ///
    /// unsafe {
    ///     assert_eq!(y.read(), 12);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn read(self) -> T
        where T: Sized,
    {
        read(self)
    }

    /// Performs a volatile read of the value from `self` without moving it. This
    /// leaves the memory in `self` unchanged.
    ///
    /// Volatile operations are intended to act on I/O memory, and are guaranteed
    /// to not be elided or reordered by the compiler across other volatile
    /// operations.
    ///
    /// # Notes
    ///
    /// Rust does not currently have a rigorously and formally defined memory model,
    /// so the precise semantics of what "volatile" means here is subject to change
    /// over time. That being said, the semantics will almost always end up pretty
    /// similar to [C11's definition of volatile][c11].
    ///
    /// The compiler shouldn't change the relative order or number of volatile
    /// memory operations. However, volatile memory operations on zero-sized types
    /// (e.g. if a zero-sized type is passed to `read_volatile`) are no-ops
    /// and may be ignored.
    ///
    /// [c11]: http://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf
    ///
    /// # Safety
    ///
    /// Beyond accepting a raw pointer, this is unsafe because it semantically
    /// moves the value out of `self` without preventing further usage of `self`.
    /// If `T` is not `Copy`, then care must be taken to ensure that the value at
    /// `self` is not used before the data is overwritten again (e.g. with `write`,
    /// `write_bytes`, or `copy`). Note that `*self = foo` counts as a use
    /// because it will attempt to drop the value previously at `*self`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let x = 12;
    /// let y = &x as *const i32;
    ///
    /// unsafe {
    ///     assert_eq!(y.read_volatile(), 12);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn read_volatile(self) -> T
        where T: Sized,
    {
        read_volatile(self)
    }

    /// Reads the value from `self` without moving it. This leaves the
    /// memory in `self` unchanged.
    ///
    /// Unlike `read`, the pointer may be unaligned.
    ///
    /// # Safety
    ///
    /// Beyond accepting a raw pointer, this is unsafe because it semantically
    /// moves the value out of `self` without preventing further usage of `self`.
    /// If `T` is not `Copy`, then care must be taken to ensure that the value at
    /// `self` is not used before the data is overwritten again (e.g. with `write`,
    /// `write_bytes`, or `copy`). Note that `*self = foo` counts as a use
    /// because it will attempt to drop the value previously at `*self`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let x = 12;
    /// let y = &x as *const i32;
    ///
    /// unsafe {
    ///     assert_eq!(y.read_unaligned(), 12);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn read_unaligned(self) -> T
        where T: Sized,
    {
        read_unaligned(self)
    }

    /// Copies `count * size_of<T>` bytes from `self` to `dest`. The source
    /// and destination may overlap.
    ///
    /// NOTE: this has the *same* argument order as `ptr::copy`.
    ///
    /// This is semantically equivalent to C's `memmove`.
    ///
    /// # Safety
    ///
    /// Care must be taken with the ownership of `self` and `dest`.
    /// This method semantically moves the values of `self` into `dest`.
    /// However it does not drop the contents of `self`, or prevent the contents
    /// of `dest` from being dropped or used.
    ///
    /// # Examples
    ///
    /// Efficiently create a Rust vector from an unsafe buffer:
    ///
    /// ```
    /// # #[allow(dead_code)]
    /// unsafe fn from_buf_raw<T: Copy>(ptr: *const T, elts: usize) -> Vec<T> {
    ///     let mut dst = Vec::with_capacity(elts);
    ///     dst.set_len(elts);
    ///     ptr.copy_to(dst.as_mut_ptr(), elts);
    ///     dst
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn copy_to(self, dest: *mut T, count: usize)
        where T: Sized,
    {
        copy(self, dest, count)
    }

    /// Copies `count * size_of<T>` bytes from `self` to `dest`. The source
    /// and destination may *not* overlap.
    ///
    /// NOTE: this has the *same* argument order as `ptr::copy_nonoverlapping`.
    ///
    /// `copy_nonoverlapping` is semantically equivalent to C's `memcpy`.
    ///
    /// # Safety
    ///
    /// Beyond requiring that the program must be allowed to access both regions
    /// of memory, it is Undefined Behavior for source and destination to
    /// overlap. Care must also be taken with the ownership of `self` and
    /// `self`. This method semantically moves the values of `self` into `dest`.
    /// However it does not drop the contents of `dest`, or prevent the contents
    /// of `self` from being dropped or used.
    ///
    /// # Examples
    ///
    /// Efficiently create a Rust vector from an unsafe buffer:
    ///
    /// ```
    /// # #[allow(dead_code)]
    /// unsafe fn from_buf_raw<T: Copy>(ptr: *const T, elts: usize) -> Vec<T> {
    ///     let mut dst = Vec::with_capacity(elts);
    ///     dst.set_len(elts);
    ///     ptr.copy_to_nonoverlapping(dst.as_mut_ptr(), elts);
    ///     dst
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn copy_to_nonoverlapping(self, dest: *mut T, count: usize)
        where T: Sized,
    {
        copy_nonoverlapping(self, dest, count)
    }

    /// Copies `count * size_of<T>` bytes from `src` to `self`. The source
    /// and destination may overlap.
    ///
    /// NOTE: this has the *opposite* argument order of `ptr::copy`.
    ///
    /// This is semantically equivalent to C's `memmove`.
    ///
    /// # Safety
    ///
    /// Care must be taken with the ownership of `src` and `self`.
    /// This method semantically moves the values of `src` into `self`.
    /// However it does not drop the contents of `self`, or prevent the contents
    /// of `src` from being dropped or used.
    ///
    /// # Examples
    ///
    /// Efficiently create a Rust vector from an unsafe buffer:
    ///
    /// ```
    /// # #[allow(dead_code)]
    /// unsafe fn from_buf_raw<T: Copy>(ptr: *const T, elts: usize) -> Vec<T> {
    ///     let mut dst: Vec<T> = Vec::with_capacity(elts);
    ///     dst.set_len(elts);
    ///     dst.as_mut_ptr().copy_from(ptr, elts);
    ///     dst
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn copy_from(self, src: *const T, count: usize)
        where T: Sized,
    {
        copy(src, self, count)
    }

    /// Copies `count * size_of<T>` bytes from `src` to `self`. The source
    /// and destination may *not* overlap.
    ///
    /// NOTE: this has the *opposite* argument order of `ptr::copy_nonoverlapping`.
    ///
    /// `copy_nonoverlapping` is semantically equivalent to C's `memcpy`.
    ///
    /// # Safety
    ///
    /// Beyond requiring that the program must be allowed to access both regions
    /// of memory, it is Undefined Behavior for source and destination to
    /// overlap. Care must also be taken with the ownership of `src` and
    /// `self`. This method semantically moves the values of `src` into `self`.
    /// However it does not drop the contents of `self`, or prevent the contents
    /// of `src` from being dropped or used.
    ///
    /// # Examples
    ///
    /// Efficiently create a Rust vector from an unsafe buffer:
    ///
    /// ```
    /// # #[allow(dead_code)]
    /// unsafe fn from_buf_raw<T: Copy>(ptr: *const T, elts: usize) -> Vec<T> {
    ///     let mut dst: Vec<T> = Vec::with_capacity(elts);
    ///     dst.set_len(elts);
    ///     dst.as_mut_ptr().copy_from_nonoverlapping(ptr, elts);
    ///     dst
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn copy_from_nonoverlapping(self, src: *const T, count: usize)
        where T: Sized,
    {
        copy_nonoverlapping(src, self, count)
    }

    /// Executes the destructor (if any) of the pointed-to value.
    ///
    /// This has two use cases:
    ///
    /// * It is *required* to use `drop_in_place` to drop unsized types like
    ///   trait objects, because they can't be read out onto the stack and
    ///   dropped normally.
    ///
    /// * It is friendlier to the optimizer to do this over `ptr::read` when
    ///   dropping manually allocated memory (e.g. when writing Box/Rc/Vec),
    ///   as the compiler doesn't need to prove that it's sound to elide the
    ///   copy.
    ///
    /// # Safety
    ///
    /// This has all the same safety problems as `ptr::read` with respect to
    /// invalid pointers, types, and double drops.
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn drop_in_place(self) {
        drop_in_place(self)
    }

    /// Overwrites a memory location with the given value without reading or
    /// dropping the old value.
    ///
    /// # Safety
    ///
    /// This operation is marked unsafe because it writes through a raw pointer.
    ///
    /// It does not drop the contents of `self`. This is safe, but it could leak
    /// allocations or resources, so care must be taken not to overwrite an object
    /// that should be dropped.
    ///
    /// Additionally, it does not drop `val`. Semantically, `val` is moved into the
    /// location pointed to by `self`.
    ///
    /// This is appropriate for initializing uninitialized memory, or overwriting
    /// memory that has previously been `read` from.
    ///
    /// The pointer must be aligned; use `write_unaligned` if that is not the case.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let mut x = 0;
    /// let y = &mut x as *mut i32;
    /// let z = 12;
    ///
    /// unsafe {
    ///     y.write(z);
    ///     assert_eq!(y.read(), 12);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn write(self, val: T)
        where T: Sized,
    {
        write(self, val)
    }

    /// Invokes memset on the specified pointer, setting `count * size_of::<T>()`
    /// bytes of memory starting at `self` to `val`.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut vec = vec![0; 4];
    /// unsafe {
    ///     let vec_ptr = vec.as_mut_ptr();
    ///     vec_ptr.write_bytes(b'a', 2);
    /// }
    /// assert_eq!(vec, [b'a', b'a', 0, 0]);
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn write_bytes(self, val: u8, count: usize)
        where T: Sized,
    {
        write_bytes(self, val, count)
    }

    /// Performs a volatile write of a memory location with the given value without
    /// reading or dropping the old value.
    ///
    /// Volatile operations are intended to act on I/O memory, and are guaranteed
    /// to not be elided or reordered by the compiler across other volatile
    /// operations.
    ///
    /// # Notes
    ///
    /// Rust does not currently have a rigorously and formally defined memory model,
    /// so the precise semantics of what "volatile" means here is subject to change
    /// over time. That being said, the semantics will almost always end up pretty
    /// similar to [C11's definition of volatile][c11].
    ///
    /// The compiler shouldn't change the relative order or number of volatile
    /// memory operations. However, volatile memory operations on zero-sized types
    /// (e.g. if a zero-sized type is passed to `write_volatile`) are no-ops
    /// and may be ignored.
    ///
    /// [c11]: http://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf
    ///
    /// # Safety
    ///
    /// This operation is marked unsafe because it accepts a raw pointer.
    ///
    /// It does not drop the contents of `self`. This is safe, but it could leak
    /// allocations or resources, so care must be taken not to overwrite an object
    /// that should be dropped.
    ///
    /// This is appropriate for initializing uninitialized memory, or overwriting
    /// memory that has previously been `read` from.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let mut x = 0;
    /// let y = &mut x as *mut i32;
    /// let z = 12;
    ///
    /// unsafe {
    ///     y.write_volatile(z);
    ///     assert_eq!(y.read_volatile(), 12);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn write_volatile(self, val: T)
        where T: Sized,
    {
        write_volatile(self, val)
    }

    /// Overwrites a memory location with the given value without reading or
    /// dropping the old value.
    ///
    /// Unlike `write`, the pointer may be unaligned.
    ///
    /// # Safety
    ///
    /// This operation is marked unsafe because it writes through a raw pointer.
    ///
    /// It does not drop the contents of `self`. This is safe, but it could leak
    /// allocations or resources, so care must be taken not to overwrite an object
    /// that should be dropped.
    ///
    /// Additionally, it does not drop `self`. Semantically, `self` is moved into the
    /// location pointed to by `val`.
    ///
    /// This is appropriate for initializing uninitialized memory, or overwriting
    /// memory that has previously been `read` from.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let mut x = 0;
    /// let y = &mut x as *mut i32;
    /// let z = 12;
    ///
    /// unsafe {
    ///     y.write_unaligned(z);
    ///     assert_eq!(y.read_unaligned(), 12);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn write_unaligned(self, val: T)
        where T: Sized,
    {
        write_unaligned(self, val)
    }

    /// Replaces the value at `self` with `src`, returning the old
    /// value, without dropping either.
    ///
    /// # Safety
    ///
    /// This is only unsafe because it accepts a raw pointer.
    /// Otherwise, this operation is identical to `mem::replace`.
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn replace(self, src: T) -> T
        where T: Sized,
    {
        replace(self, src)
    }

    /// Swaps the values at two mutable locations of the same type, without
    /// deinitializing either. They may overlap, unlike `mem::swap` which is
    /// otherwise equivalent.
    ///
    /// # Safety
    ///
    /// This function copies the memory through the raw pointers passed to it
    /// as arguments.
    ///
    /// Ensure that these pointers are valid before calling `swap`.
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn swap(self, with: *mut T)
        where T: Sized,
    {
        swap(self, with)
    }

    /// Computes the offset that needs to be applied to the pointer in order to make it aligned to
    /// `align`.
    ///
    /// If it is not possible to align the pointer, the implementation returns
    /// `usize::max_value()`.
    ///
    /// The offset is expressed in number of `T` elements, and not bytes. The value returned can be
    /// used with the `offset` or `offset_to` methods.
    ///
    /// There are no guarantees whatsover that offsetting the pointer will not overflow or go
    /// beyond the allocation that the pointer points into. It is up to the caller to ensure that
    /// the returned offset is correct in all terms other than alignment.
    ///
    /// # Panics
    ///
    /// The function panics if `align` is not a power-of-two.
    ///
    /// # Examples
    ///
    /// Accessing adjacent `u8` as `u16`
    ///
    /// ```
    /// # #![feature(align_offset)]
    /// # fn foo(n: usize) {
    /// # use std::mem::align_of;
    /// # unsafe {
    /// let x = [5u8, 6u8, 7u8, 8u8, 9u8];
    /// let ptr = &x[n] as *const u8;
    /// let offset = ptr.align_offset(align_of::<u16>());
    /// if offset < x.len() - n - 1 {
    ///     let u16_ptr = ptr.offset(offset as isize) as *const u16;
    ///     assert_ne!(*u16_ptr, 500);
    /// } else {
    ///     // while the pointer can be aligned via `offset`, it would point
    ///     // outside the allocation
    /// }
    /// # } }
    /// ```
    #[unstable(feature = "align_offset", issue = "44488")]
    #[cfg(not(stage0))]
    pub fn align_offset(self, align: usize) -> usize where T: Sized {
        if !align.is_power_of_two() {
            panic!("align_offset: align is not a power-of-two");
        }
        unsafe {
            align_offset(self, align)
        }
    }

    /// definitely docs.
    #[unstable(feature = "align_offset", issue = "44488")]
    #[cfg(stage0)]
    pub fn align_offset(self, align: usize) -> usize where T: Sized {
        if !align.is_power_of_two() {
            panic!("align_offset: align is not a power-of-two");
        }
        unsafe {
            intrinsics::align_offset(self as *const (), align)
        }
    }
}

/// Align pointer `p`.
///
/// Calculate offset (in terms of elements of `stride` stride) that has to be applied
/// to pointer `p` so that pointer `p` would get aligned to `a`.
///
/// Note: This implementation has been carefully tailored to not panic. It is UB for this to panic.
/// The only real change that can be made here is change of `INV_TABLE_MOD_16` and associated
/// constants.
///
/// If we ever decide to make it possible to call the intrinsic with `a` that is not a
/// power-of-two, it will probably be more prudent to just change to a naive implementation rather
/// than trying to adapt this to accomodate that change.
///
/// Any questions go to @nagisa.
#[lang="align_offset"]
#[cfg(not(stage0))]
pub(crate) unsafe fn align_offset<T: Sized>(p: *const T, a: usize) -> usize {
    /// Calculate multiplicative modular inverse of `x` modulo `m`.
    ///
    /// This implementation is tailored for align_offset and has following preconditions:
    ///
    /// * `m` is a power-of-two;
    /// * `x < m`; (if `x ≥ m`, pass in `x % m` instead)
    ///
    /// Implementation of this function shall not panic. Ever.
    #[inline]
    fn mod_inv(x: usize, m: usize) -> usize {
        /// Multiplicative modular inverse table modulo 2⁴ = 16.
        ///
        /// Note, that this table does not contain values where inverse does not exist (i.e. for
        /// `0⁻¹ mod 16`, `2⁻¹ mod 16`, etc.)
        const INV_TABLE_MOD_16: [usize; 8] = [1, 11, 13, 7, 9, 3, 5, 15];
        /// Modulo for which the `INV_TABLE_MOD_16` is intended.
        const INV_TABLE_MOD: usize = 16;
        /// INV_TABLE_MOD²
        const INV_TABLE_MOD_SQUARED: usize = INV_TABLE_MOD * INV_TABLE_MOD;

        let table_inverse = INV_TABLE_MOD_16[(x & (INV_TABLE_MOD - 1)) >> 1];
        if m <= INV_TABLE_MOD {
            return table_inverse & (m - 1);
        } else {
            // We iterate "up" using the following formula:
            //
            // $$ xy ≡ 1 (mod 2ⁿ) → xy (2 - xy) ≡ 1 (mod 2²ⁿ) $$
            //
            // until 2²ⁿ ≥ m. Then we can reduce to our desired `m` by taking the result `mod m`.
            let mut inverse = table_inverse;
            let mut going_mod = INV_TABLE_MOD_SQUARED;
            loop {
                // y = y * (2 - xy) mod n
                //
                // Note, that we use wrapping operations here intentionally – the original formula
                // uses e.g. subtraction `mod n`. It is entirely fine to do them `mod
                // usize::max_value()` instead, because we take the result `mod n` at the end
                // anyway.
                inverse = inverse.wrapping_mul(
                    2usize.wrapping_sub(x.wrapping_mul(inverse))
                ) & (going_mod - 1);
                if going_mod > m {
                    return inverse & (m - 1);
                }
                going_mod = going_mod.wrapping_mul(going_mod);
            }
        }
    }

    let stride = ::mem::size_of::<T>();
    let a_minus_one = a.wrapping_sub(1);
    let pmoda = p as usize & a_minus_one;

    if pmoda == 0 {
        // Already aligned. Yay!
        return 0;
    }

    if stride <= 1 {
        return if stride == 0 {
            // If the pointer is not aligned, and the element is zero-sized, then no amount of
            // elements will ever align the pointer.
            !0
        } else {
            a.wrapping_sub(pmoda)
        };
    }

    let smoda = stride & a_minus_one;
    // a is power-of-two so cannot be 0. stride = 0 is handled above.
    let gcdpow = intrinsics::cttz_nonzero(stride).min(intrinsics::cttz_nonzero(a));
    let gcd = 1usize << gcdpow;

    if gcd == 1 {
        // This branch solves for the variable $o$ in following linear congruence equation:
        //
        // ⎰ p + o ≡ 0 (mod a)   # $p + o$ must be aligned to specified alignment $a$
        // ⎱     o ≡ 0 (mod s)   # offset $o$ must be a multiple of stride $s$
        //
        // where
        //
        // * a, s are co-prime
        //
        // This gives us the formula below:
        //
        // o = (a - (p mod a)) * (s⁻¹ mod a) * s
        //
        // The first term is “the relative alignment of p to a”, the second term is “how does
        // incrementing p by one s change the relative alignment of p”, the third term is
        // translating change in units of s to a byte count.
        //
        // Furthermore, the result produced by this solution is not “minimal”, so it is necessary
        // to take the result $o mod lcm(s, a)$. Since $s$ and $a$ are co-prime (i.e. $gcd(s, a) =
        // 1$) and $lcm(s, a) = s * a / gcd(s, a)$, we can replace $lcm(s, a)$ with just a $s * a$.
        //
        // (Author note: we decided later on to express the offset in "elements" rather than bytes,
        // which drops the multiplication by `s` on both sides of the modulo.)
        return intrinsics::unchecked_rem(a.wrapping_sub(pmoda).wrapping_mul(mod_inv(smoda, a)), a);
    }

    if p as usize & (gcd - 1) == 0 {
        // This can be aligned, but `a` and `stride` are not co-prime, so a somewhat adapted
        // formula is used.
        let j = a.wrapping_sub(pmoda) >> gcdpow;
        let k = smoda >> gcdpow;
        return intrinsics::unchecked_rem(j.wrapping_mul(mod_inv(k, a)), a >> gcdpow);
    }

    // Cannot be aligned at all.
    return usize::max_value();
}



// Equality for pointers
#[stable(feature = "rust1", since = "1.0.0")]
impl<T: ?Sized> PartialEq for *const T {
    #[inline]
    fn eq(&self, other: &*const T) -> bool { *self == *other }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<T: ?Sized> Eq for *const T {}

#[stable(feature = "rust1", since = "1.0.0")]
impl<T: ?Sized> PartialEq for *mut T {
    #[inline]
    fn eq(&self, other: &*mut T) -> bool { *self == *other }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<T: ?Sized> Eq for *mut T {}

/// Compare raw pointers for equality.
///
/// This is the same as using the `==` operator, but less generic:
/// the arguments have to be `*const T` raw pointers,
/// not anything that implements `PartialEq`.
///
/// This can be used to compare `&T` references (which coerce to `*const T` implicitly)
/// by their address rather than comparing the values they point to
/// (which is what the `PartialEq for &T` implementation does).
///
/// # Examples
///
/// ```
/// use std::ptr;
///
/// let five = 5;
/// let other_five = 5;
/// let five_ref = &five;
/// let same_five_ref = &five;
/// let other_five_ref = &other_five;
///
/// assert!(five_ref == same_five_ref);
/// assert!(five_ref == other_five_ref);
///
/// assert!(ptr::eq(five_ref, same_five_ref));
/// assert!(!ptr::eq(five_ref, other_five_ref));
/// ```
#[stable(feature = "ptr_eq", since = "1.17.0")]
#[inline]
pub fn eq<T: ?Sized>(a: *const T, b: *const T) -> bool {
    a == b
}

// Impls for function pointers
macro_rules! fnptr_impls_safety_abi {
    ($FnTy: ty, $($Arg: ident),*) => {
        #[stable(feature = "fnptr_impls", since = "1.4.0")]
        impl<Ret, $($Arg),*> PartialEq for $FnTy {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                *self as usize == *other as usize
            }
        }

        #[stable(feature = "fnptr_impls", since = "1.4.0")]
        impl<Ret, $($Arg),*> Eq for $FnTy {}

        #[stable(feature = "fnptr_impls", since = "1.4.0")]
        impl<Ret, $($Arg),*> PartialOrd for $FnTy {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                (*self as usize).partial_cmp(&(*other as usize))
            }
        }

        #[stable(feature = "fnptr_impls", since = "1.4.0")]
        impl<Ret, $($Arg),*> Ord for $FnTy {
            #[inline]
            fn cmp(&self, other: &Self) -> Ordering {
                (*self as usize).cmp(&(*other as usize))
            }
        }

        #[stable(feature = "fnptr_impls", since = "1.4.0")]
        impl<Ret, $($Arg),*> hash::Hash for $FnTy {
            fn hash<HH: hash::Hasher>(&self, state: &mut HH) {
                state.write_usize(*self as usize)
            }
        }

        #[stable(feature = "fnptr_impls", since = "1.4.0")]
        impl<Ret, $($Arg),*> fmt::Pointer for $FnTy {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::Pointer::fmt(&(*self as *const ()), f)
            }
        }

        #[stable(feature = "fnptr_impls", since = "1.4.0")]
        impl<Ret, $($Arg),*> fmt::Debug for $FnTy {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::Pointer::fmt(&(*self as *const ()), f)
            }
        }
    }
}

macro_rules! fnptr_impls_args {
    ($($Arg: ident),+) => {
        fnptr_impls_safety_abi! { extern "Rust" fn($($Arg),*) -> Ret, $($Arg),* }
        fnptr_impls_safety_abi! { extern "C" fn($($Arg),*) -> Ret, $($Arg),* }
        fnptr_impls_safety_abi! { extern "C" fn($($Arg),* , ...) -> Ret, $($Arg),* }
        fnptr_impls_safety_abi! { unsafe extern "Rust" fn($($Arg),*) -> Ret, $($Arg),* }
        fnptr_impls_safety_abi! { unsafe extern "C" fn($($Arg),*) -> Ret, $($Arg),* }
        fnptr_impls_safety_abi! { unsafe extern "C" fn($($Arg),* , ...) -> Ret, $($Arg),* }
    };
    () => {
        // No variadic functions with 0 parameters
        fnptr_impls_safety_abi! { extern "Rust" fn() -> Ret, }
        fnptr_impls_safety_abi! { extern "C" fn() -> Ret, }
        fnptr_impls_safety_abi! { unsafe extern "Rust" fn() -> Ret, }
        fnptr_impls_safety_abi! { unsafe extern "C" fn() -> Ret, }
    };
}

fnptr_impls_args! { }
fnptr_impls_args! { A }
fnptr_impls_args! { A, B }
fnptr_impls_args! { A, B, C }
fnptr_impls_args! { A, B, C, D }
fnptr_impls_args! { A, B, C, D, E }
fnptr_impls_args! { A, B, C, D, E, F }
fnptr_impls_args! { A, B, C, D, E, F, G }
fnptr_impls_args! { A, B, C, D, E, F, G, H }
fnptr_impls_args! { A, B, C, D, E, F, G, H, I }
fnptr_impls_args! { A, B, C, D, E, F, G, H, I, J }
fnptr_impls_args! { A, B, C, D, E, F, G, H, I, J, K }
fnptr_impls_args! { A, B, C, D, E, F, G, H, I, J, K, L }

// Comparison for pointers
#[stable(feature = "rust1", since = "1.0.0")]
impl<T: ?Sized> Ord for *const T {
    #[inline]
    fn cmp(&self, other: &*const T) -> Ordering {
        if self < other {
            Less
        } else if self == other {
            Equal
        } else {
            Greater
        }
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<T: ?Sized> PartialOrd for *const T {
    #[inline]
    fn partial_cmp(&self, other: &*const T) -> Option<Ordering> {
        Some(self.cmp(other))
    }

    #[inline]
    fn lt(&self, other: &*const T) -> bool { *self < *other }

    #[inline]
    fn le(&self, other: &*const T) -> bool { *self <= *other }

    #[inline]
    fn gt(&self, other: &*const T) -> bool { *self > *other }

    #[inline]
    fn ge(&self, other: &*const T) -> bool { *self >= *other }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<T: ?Sized> Ord for *mut T {
    #[inline]
    fn cmp(&self, other: &*mut T) -> Ordering {
        if self < other {
            Less
        } else if self == other {
            Equal
        } else {
            Greater
        }
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<T: ?Sized> PartialOrd for *mut T {
    #[inline]
    fn partial_cmp(&self, other: &*mut T) -> Option<Ordering> {
        Some(self.cmp(other))
    }

    #[inline]
    fn lt(&self, other: &*mut T) -> bool { *self < *other }

    #[inline]
    fn le(&self, other: &*mut T) -> bool { *self <= *other }

    #[inline]
    fn gt(&self, other: &*mut T) -> bool { *self > *other }

    #[inline]
    fn ge(&self, other: &*mut T) -> bool { *self >= *other }
}

/// A wrapper around a raw non-null `*mut T` that indicates that the possessor
/// of this wrapper owns the referent. Useful for building abstractions like
/// `Box<T>`, `Vec<T>`, `String`, and `HashMap<K, V>`.
///
/// Unlike `*mut T`, `Unique<T>` behaves "as if" it were an instance of `T`.
/// It implements `Send`/`Sync` if `T` is `Send`/`Sync`. It also implies
/// the kind of strong aliasing guarantees an instance of `T` can expect:
/// the referent of the pointer should not be modified without a unique path to
/// its owning Unique.
///
/// If you're uncertain of whether it's correct to use `Unique` for your purposes,
/// consider using `NonNull`, which has weaker semantics.
///
/// Unlike `*mut T`, the pointer must always be non-null, even if the pointer
/// is never dereferenced. This is so that enums may use this forbidden value
/// as a discriminant -- `Option<Unique<T>>` has the same size as `Unique<T>`.
/// However the pointer may still dangle if it isn't dereferenced.
///
/// Unlike `*mut T`, `Unique<T>` is covariant over `T`. This should always be correct
/// for any type which upholds Unique's aliasing requirements.
#[unstable(feature = "ptr_internals", issue = "0",
           reason = "use NonNull instead and consider PhantomData<T> \
                     (if you also use #[may_dangle]), Send, and/or Sync")]
#[doc(hidden)]
pub struct Unique<T: ?Sized> {
    pointer: NonZero<*const T>,
    // NOTE: this marker has no consequences for variance, but is necessary
    // for dropck to understand that we logically own a `T`.
    //
    // For details, see:
    // https://github.com/rust-lang/rfcs/blob/master/text/0769-sound-generic-drop.md#phantom-data
    _marker: PhantomData<T>,
}

#[unstable(feature = "ptr_internals", issue = "0")]
impl<T: ?Sized> fmt::Debug for Unique<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.as_ptr(), f)
    }
}

/// `Unique` pointers are `Send` if `T` is `Send` because the data they
/// reference is unaliased. Note that this aliasing invariant is
/// unenforced by the type system; the abstraction using the
/// `Unique` must enforce it.
#[unstable(feature = "ptr_internals", issue = "0")]
unsafe impl<T: Send + ?Sized> Send for Unique<T> { }

/// `Unique` pointers are `Sync` if `T` is `Sync` because the data they
/// reference is unaliased. Note that this aliasing invariant is
/// unenforced by the type system; the abstraction using the
/// `Unique` must enforce it.
#[unstable(feature = "ptr_internals", issue = "0")]
unsafe impl<T: Sync + ?Sized> Sync for Unique<T> { }

#[unstable(feature = "ptr_internals", issue = "0")]
impl<T: Sized> Unique<T> {
    /// Creates a new `Unique` that is dangling, but well-aligned.
    ///
    /// This is useful for initializing types which lazily allocate, like
    /// `Vec::new` does.
    // FIXME: rename to dangling() to match NonNull?
    pub const fn empty() -> Self {
        unsafe {
            Unique::new_unchecked(mem::align_of::<T>() as *mut T)
        }
    }
}

#[unstable(feature = "ptr_internals", issue = "0")]
impl<T: ?Sized> Unique<T> {
    /// Creates a new `Unique`.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null.
    pub const unsafe fn new_unchecked(ptr: *mut T) -> Self {
        Unique { pointer: NonZero(ptr as _), _marker: PhantomData }
    }

    /// Creates a new `Unique` if `ptr` is non-null.
    pub fn new(ptr: *mut T) -> Option<Self> {
        if !ptr.is_null() {
            Some(Unique { pointer: NonZero(ptr as _), _marker: PhantomData })
        } else {
            None
        }
    }

    /// Acquires the underlying `*mut` pointer.
    pub fn as_ptr(self) -> *mut T {
        self.pointer.0 as *mut T
    }

    /// Dereferences the content.
    ///
    /// The resulting lifetime is bound to self so this behaves "as if"
    /// it were actually an instance of T that is getting borrowed. If a longer
    /// (unbound) lifetime is needed, use `&*my_ptr.as_ptr()`.
    pub unsafe fn as_ref(&self) -> &T {
        &*self.as_ptr()
    }

    /// Mutably dereferences the content.
    ///
    /// The resulting lifetime is bound to self so this behaves "as if"
    /// it were actually an instance of T that is getting borrowed. If a longer
    /// (unbound) lifetime is needed, use `&mut *my_ptr.as_ptr()`.
    pub unsafe fn as_mut(&mut self) -> &mut T {
        &mut *self.as_ptr()
    }
}

#[unstable(feature = "ptr_internals", issue = "0")]
impl<T: ?Sized> Clone for Unique<T> {
    fn clone(&self) -> Self {
        *self
    }
}

#[unstable(feature = "ptr_internals", issue = "0")]
impl<T: ?Sized> Copy for Unique<T> { }

#[unstable(feature = "ptr_internals", issue = "0")]
impl<T: ?Sized, U: ?Sized> CoerceUnsized<Unique<U>> for Unique<T> where T: Unsize<U> { }

#[unstable(feature = "ptr_internals", issue = "0")]
impl<T: ?Sized> fmt::Pointer for Unique<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.as_ptr(), f)
    }
}

#[unstable(feature = "ptr_internals", issue = "0")]
impl<'a, T: ?Sized> From<&'a mut T> for Unique<T> {
    fn from(reference: &'a mut T) -> Self {
        Unique { pointer: NonZero(reference as _), _marker: PhantomData }
    }
}

#[unstable(feature = "ptr_internals", issue = "0")]
impl<'a, T: ?Sized> From<&'a T> for Unique<T> {
    fn from(reference: &'a T) -> Self {
        Unique { pointer: NonZero(reference as _), _marker: PhantomData }
    }
}

#[unstable(feature = "ptr_internals", issue = "0")]
impl<'a, T: ?Sized> From<NonNull<T>> for Unique<T> {
    fn from(p: NonNull<T>) -> Self {
        Unique { pointer: p.pointer, _marker: PhantomData }
    }
}

/// `*mut T` but non-zero and covariant.
///
/// This is often the correct thing to use when building data structures using
/// raw pointers, but is ultimately more dangerous to use because of its additional
/// properties. If you're not sure if you should use `NonNull<T>`, just use `*mut T`!
///
/// Unlike `*mut T`, the pointer must always be non-null, even if the pointer
/// is never dereferenced. This is so that enums may use this forbidden value
/// as a discriminant -- `Option<NonNull<T>>` has the same size as `*mut T`.
/// However the pointer may still dangle if it isn't dereferenced.
///
/// Unlike `*mut T`, `NonNull<T>` is covariant over `T`. If this is incorrect
/// for your use case, you should include some PhantomData in your type to
/// provide invariance, such as `PhantomData<Cell<T>>` or `PhantomData<&'a mut T>`.
/// Usually this won't be necessary; covariance is correct for most safe abstractions,
/// such as Box, Rc, Arc, Vec, and LinkedList. This is the case because they
/// provide a public API that follows the normal shared XOR mutable rules of Rust.
#[stable(feature = "nonnull", since = "1.25.0")]
pub struct NonNull<T: ?Sized> {
    pointer: NonZero<*const T>,
}

/// `NonNull` pointers are not `Send` because the data they reference may be aliased.
// NB: This impl is unnecessary, but should provide better error messages.
#[stable(feature = "nonnull", since = "1.25.0")]
impl<T: ?Sized> !Send for NonNull<T> { }

/// `NonNull` pointers are not `Sync` because the data they reference may be aliased.
// NB: This impl is unnecessary, but should provide better error messages.
#[stable(feature = "nonnull", since = "1.25.0")]
impl<T: ?Sized> !Sync for NonNull<T> { }

impl<T: Sized> NonNull<T> {
    /// Creates a new `NonNull` that is dangling, but well-aligned.
    ///
    /// This is useful for initializing types which lazily allocate, like
    /// `Vec::new` does.
    #[stable(feature = "nonnull", since = "1.25.0")]
    pub fn dangling() -> Self {
        unsafe {
            let ptr = mem::align_of::<T>() as *mut T;
            NonNull::new_unchecked(ptr)
        }
    }
}

impl<T: ?Sized> NonNull<T> {
    /// Creates a new `NonNull`.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null.
    #[stable(feature = "nonnull", since = "1.25.0")]
    pub const unsafe fn new_unchecked(ptr: *mut T) -> Self {
        NonNull { pointer: NonZero(ptr as _) }
    }

    /// Creates a new `NonNull` if `ptr` is non-null.
    #[stable(feature = "nonnull", since = "1.25.0")]
    pub fn new(ptr: *mut T) -> Option<Self> {
        if !ptr.is_null() {
            Some(NonNull { pointer: NonZero(ptr as _) })
        } else {
            None
        }
    }

    /// Acquires the underlying `*mut` pointer.
    #[stable(feature = "nonnull", since = "1.25.0")]
    pub fn as_ptr(self) -> *mut T {
        self.pointer.0 as *mut T
    }

    /// Dereferences the content.
    ///
    /// The resulting lifetime is bound to self so this behaves "as if"
    /// it were actually an instance of T that is getting borrowed. If a longer
    /// (unbound) lifetime is needed, use `&*my_ptr.as_ptr()`.
    #[stable(feature = "nonnull", since = "1.25.0")]
    pub unsafe fn as_ref(&self) -> &T {
        &*self.as_ptr()
    }

    /// Mutably dereferences the content.
    ///
    /// The resulting lifetime is bound to self so this behaves "as if"
    /// it were actually an instance of T that is getting borrowed. If a longer
    /// (unbound) lifetime is needed, use `&mut *my_ptr.as_ptr()`.
    #[stable(feature = "nonnull", since = "1.25.0")]
    pub unsafe fn as_mut(&mut self) -> &mut T {
        &mut *self.as_ptr()
    }

    /// Cast to a pointer of another type
    #[stable(feature = "nonnull_cast", since = "1.27.0")]
    pub fn cast<U>(self) -> NonNull<U> {
        unsafe {
            NonNull::new_unchecked(self.as_ptr() as *mut U)
        }
    }
}

#[stable(feature = "nonnull", since = "1.25.0")]
impl<T: ?Sized> Clone for NonNull<T> {
    fn clone(&self) -> Self {
        *self
    }
}

#[stable(feature = "nonnull", since = "1.25.0")]
impl<T: ?Sized> Copy for NonNull<T> { }

#[unstable(feature = "coerce_unsized", issue = "27732")]
impl<T: ?Sized, U: ?Sized> CoerceUnsized<NonNull<U>> for NonNull<T> where T: Unsize<U> { }

#[stable(feature = "nonnull", since = "1.25.0")]
impl<T: ?Sized> fmt::Debug for NonNull<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.as_ptr(), f)
    }
}

#[stable(feature = "nonnull", since = "1.25.0")]
impl<T: ?Sized> fmt::Pointer for NonNull<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.as_ptr(), f)
    }
}

#[stable(feature = "nonnull", since = "1.25.0")]
impl<T: ?Sized> Eq for NonNull<T> {}

#[stable(feature = "nonnull", since = "1.25.0")]
impl<T: ?Sized> PartialEq for NonNull<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_ptr() == other.as_ptr()
    }
}

#[stable(feature = "nonnull", since = "1.25.0")]
impl<T: ?Sized> Ord for NonNull<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_ptr().cmp(&other.as_ptr())
    }
}

#[stable(feature = "nonnull", since = "1.25.0")]
impl<T: ?Sized> PartialOrd for NonNull<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_ptr().partial_cmp(&other.as_ptr())
    }
}

#[stable(feature = "nonnull", since = "1.25.0")]
impl<T: ?Sized> hash::Hash for NonNull<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_ptr().hash(state)
    }
}

#[unstable(feature = "ptr_internals", issue = "0")]
impl<T: ?Sized> From<Unique<T>> for NonNull<T> {
    fn from(unique: Unique<T>) -> Self {
        NonNull { pointer: unique.pointer }
    }
}

#[stable(feature = "nonnull", since = "1.25.0")]
impl<'a, T: ?Sized> From<&'a mut T> for NonNull<T> {
    fn from(reference: &'a mut T) -> Self {
        NonNull { pointer: NonZero(reference as _) }
    }
}

#[stable(feature = "nonnull", since = "1.25.0")]
impl<'a, T: ?Sized> From<&'a T> for NonNull<T> {
    fn from(reference: &'a T) -> Self {
        NonNull { pointer: NonZero(reference as _) }
    }
}
