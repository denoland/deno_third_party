use std::mem;
use std::panic::RefUnwindSafe;
use std::ops::{Add, BitAnd, BitOr, BitXor, Sub};
use std::num::Wrapping;
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};

pub trait IntCast : Copy + Eq + Add<Output=Self> + BitAnd<Output=Self>
    + BitOr<Output=Self> + BitXor<Output=Self> + Sub<Output=Self> {
    type Public: PartialEq + Copy;

    fn from(u: usize) -> Self;
    fn to(self) -> usize;

    fn new(p: Self::Public) -> Self;
    fn unwrap(self) -> Self::Public;
}

macro_rules! intcast {
    ($($type:ident)+) => {
        $(
            impl IntCast for Wrapping<$type> {
                type Public = $type;

                fn from(u: usize) -> Self {
                    Wrapping(u as $type)
                }
                fn to(self) -> usize {
                    self.0 as usize
                }

                fn new(p: $type) -> Self {
                    Wrapping(p)
                }

                fn unwrap(self) -> $type {
                    self.0
                }
            }
        )+
    }
}
intcast! { u8 i8 u16 i16 u32 i32 u64 i64 }

pub struct Template<T> {
    v: UnsafeCell<T>,
}

impl<T: Default + IntCast> Default for Template<T> {
    fn default() -> Self {
        Self::new(T::default().unwrap())
    }
}

// TODO: impl Debug

unsafe impl<T> Sync for Template<T> {}
impl<T> RefUnwindSafe for Template<T> {}

fn inject<T>(a: usize, b: usize, offset: usize) -> usize {
    let mask = ((1 << (mem::size_of::<T>() * 8)) - 1) << offset;
    (a & !mask) | (b << offset)
}

// straight from libcore's atomic.rs
#[inline]
fn strongest_failure_ordering(order: Ordering) -> Ordering {
    use self::Ordering::*;
    match order {
        Release => Relaxed,
        Relaxed => Relaxed,
        SeqCst => SeqCst,
        Acquire => Acquire,
        AcqRel => Acquire,
        _ => unreachable!(),
    }
}

impl<T: IntCast> Template<T> {
    #[inline]
    fn proxy(&self) -> (&AtomicUsize, usize) {
        let ptr = self.v.get() as usize;
        let aligned = ptr & !(mem::size_of::<usize>() - 1);
        (unsafe { &*(aligned as *const AtomicUsize) }, (ptr - aligned) * 8)
    }

    // TODO: make this const if const is stable first
    #[inline]
    pub /*const*/ fn new(v: T::Public) -> Self {
        Template { v: UnsafeCell::new(T::new(v)) }
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut T::Public {
        unsafe { &mut *(self.v.get() as *mut T::Public) }
    }

    #[inline]
    pub fn into_inner(self) -> T::Public {
        unsafe { self.v.into_inner() }.unwrap()
    }

    #[inline]
    pub fn load(&self, order: Ordering) -> T::Public {
        let (p, o) = self.proxy();
        T::from(p.load(order) >> o).unwrap()
    }

    #[inline]
    fn op<F: Fn(T) -> Option<T>>(&self, f: F, order: Ordering) -> T::Public {
        self.op_new(f, order, strongest_failure_ordering(order))
    }

    #[inline]
    fn op_new<F: Fn(T) -> Option<T>>(&self, f: F, success: Ordering, failure: Ordering) -> T::Public {
        let (p, o) = self.proxy();
        let mut old = p.load(Ordering::Relaxed);
        loop {
            let old_t = T::from(old >> o);
            let new_t = match f(old_t) {
                Some(x) => x,
                None => return old_t.unwrap(),
            };

            match Self::op_weak(p, o, old, new_t, success, failure) {
                Ok(()) => return T::from(old >> o).unwrap(),
                Err(prev) => old = prev,
            };
        }
    }

    #[inline]
    fn op_weak(p: &AtomicUsize, o: usize, old: usize, new_t: T, success: Ordering, failure: Ordering) -> Result<(), usize> {
        let new = inject::<T>(old, new_t.to(), o);
        p.compare_exchange_weak(old, new, success, failure).map(|_| ())
    }
    
    #[inline]
    pub fn store(&self, val: T::Public, order: Ordering) {
        self.op(|_| Some(T::new(val)), order);
    }

    #[inline]
    pub fn swap(&self, val: T::Public, order: Ordering) -> T::Public {
        self.op(|_| Some(T::new(val)), order)
    }

    #[inline]
    pub fn compare_and_swap(&self, current: T::Public, new: T::Public, order: Ordering) -> T::Public {
        self.op(|x| if x == T::new(current) { Some(T::new(new)) } else { None }, order)
    }

    #[inline]
    pub fn compare_exchange(&self, current: T::Public, new: T::Public, success: Ordering, failure: Ordering) -> Result<T::Public, T::Public> {
        match self.op_new(|x| if x == T::new(current) { Some(T::new(new)) } else { None }, success, failure) {
            x if x == current => Ok(x),
            x => Err(x),
        }
    }

    #[inline]
    pub fn compare_exchange_weak(&self, current: T::Public, new: T::Public, success: Ordering, failure: Ordering) -> Result<T::Public, T::Public> {
        let (p, o) = self.proxy();
        let old = p.load(Ordering::Relaxed);
        let old_t = T::from(old >> o).unwrap();
        if old_t != current {
            return Err(old_t);
        }

        Self::op_weak(p, o, old, T::new(new), success, failure).map(|()| current).map_err(|x| T::from(x >> o).unwrap())
    }

    #[inline]
    pub fn fetch_add(&self, val: T::Public, order: Ordering) -> T::Public {
        self.op(|x| Some(x + T::new(val)), order)
    }

    #[inline]
    pub fn fetch_sub(&self, val: T::Public, order: Ordering) -> T::Public {
        self.op(|x| Some(x - T::new(val)), order)
    }

    #[inline]
    pub fn fetch_and(&self, val: T::Public, order: Ordering) -> T::Public {
        self.op(|x| Some(x & T::new(val)), order)
    }

    #[inline]
    pub fn fetch_or(&self, val: T::Public, order: Ordering) -> T::Public {
        self.op(|x| Some(x | T::new(val)), order)
    }

    #[inline]
    pub fn fetch_xor(&self, val: T::Public, order: Ordering) -> T::Public {
        self.op(|x| Some(x ^ T::new(val)), order)
    }
}

pub type AtomicI8 = Template<Wrapping<i8>>;
pub type AtomicU8 = Template<Wrapping<u8>>;
pub type AtomicI16 = Template<Wrapping<i16>>;
pub type AtomicU16 = Template<Wrapping<u16>>;
pub type AtomicI32 = Template<Wrapping<i32>>;
pub type AtomicU32 = Template<Wrapping<u32>>;
pub type AtomicI64 = Template<Wrapping<i64>>;
pub type AtomicU64 = Template<Wrapping<u64>>;


#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;
    use std::u16;

    #[test]
    fn basics() {
        let v = AtomicU16::new(1337);
        let o = Ordering::Relaxed;
        assert_eq!(v.swap(42, o), 1337);
        assert_eq!(v.fetch_add(1, o), 42);
        assert_eq!(v.fetch_sub(1, o), 43);
        assert_eq!(v.fetch_and(0x20, o), 42);
        assert_eq!(v.fetch_or(0x0a, o), 0x20);
        assert_eq!(v.fetch_xor(42, o), 42);
        assert_eq!(v.fetch_sub(1, o), 0);
        assert_eq!(v.fetch_add(1, o), u16::MAX);
        assert_eq!(v.compare_and_swap(1, 2, o), 0);
        assert_eq!(v.compare_and_swap(0, 3, o), 0);
        assert_eq!(v.load(o), 3);
    }
}
