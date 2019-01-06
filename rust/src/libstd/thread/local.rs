//! Thread local storage

#![unstable(feature = "thread_local_internals", issue = "0")]

use cell::UnsafeCell;
use fmt;
use hint;
use mem;

/// A thread local storage key which owns its contents.
///
/// This key uses the fastest possible implementation available to it for the
/// target platform. It is instantiated with the [`thread_local!`] macro and the
/// primary method is the [`with`] method.
///
/// The [`with`] method yields a reference to the contained value which cannot be
/// sent across threads or escape the given closure.
///
/// # Initialization and Destruction
///
/// Initialization is dynamically performed on the first call to [`with`]
/// within a thread, and values that implement [`Drop`] get destructed when a
/// thread exits. Some caveats apply, which are explained below.
///
/// A `LocalKey`'s initializer cannot recursively depend on itself, and using
/// a `LocalKey` in this way will cause the initializer to infinitely recurse
/// on the first call to `with`.
///
/// # Examples
///
/// ```
/// use std::cell::RefCell;
/// use std::thread;
///
/// thread_local!(static FOO: RefCell<u32> = RefCell::new(1));
///
/// FOO.with(|f| {
///     assert_eq!(*f.borrow(), 1);
///     *f.borrow_mut() = 2;
/// });
///
/// // each thread starts out with the initial value of 1
/// thread::spawn(move|| {
///     FOO.with(|f| {
///         assert_eq!(*f.borrow(), 1);
///         *f.borrow_mut() = 3;
///     });
/// });
///
/// // we retain our original value of 2 despite the child thread
/// FOO.with(|f| {
///     assert_eq!(*f.borrow(), 2);
/// });
/// ```
///
/// # Platform-specific behavior
///
/// Note that a "best effort" is made to ensure that destructors for types
/// stored in thread local storage are run, but not all platforms can guarantee
/// that destructors will be run for all types in thread local storage. For
/// example, there are a number of known caveats where destructors are not run:
///
/// 1. On Unix systems when pthread-based TLS is being used, destructors will
///    not be run for TLS values on the main thread when it exits. Note that the
///    application will exit immediately after the main thread exits as well.
/// 2. On all platforms it's possible for TLS to re-initialize other TLS slots
///    during destruction. Some platforms ensure that this cannot happen
///    infinitely by preventing re-initialization of any slot that has been
///    destroyed, but not all platforms have this guard. Those platforms that do
///    not guard typically have a synthetic limit after which point no more
///    destructors are run.
/// 3. On macOS, initializing TLS during destruction of other TLS slots can
///    sometimes cancel *all* destructors for the current thread, whether or not
///    the slots have already had their destructors run or not.
///
/// [`with`]: ../../std/thread/struct.LocalKey.html#method.with
/// [`thread_local!`]: ../../std/macro.thread_local.html
/// [`Drop`]: ../../std/ops/trait.Drop.html
#[stable(feature = "rust1", since = "1.0.0")]
pub struct LocalKey<T: 'static> {
    // This outer `LocalKey<T>` type is what's going to be stored in statics,
    // but actual data inside will sometimes be tagged with #[thread_local].
    // It's not valid for a true static to reference a #[thread_local] static,
    // so we get around that by exposing an accessor through a layer of function
    // indirection (this thunk).
    //
    // Note that the thunk is itself unsafe because the returned lifetime of the
    // slot where data lives, `'static`, is not actually valid. The lifetime
    // here is actually slightly shorter than the currently running thread!
    //
    // Although this is an extra layer of indirection, it should in theory be
    // trivially devirtualizable by LLVM because the value of `inner` never
    // changes and the constant should be readonly within a crate. This mainly
    // only runs into problems when TLS statics are exported across crates.
    inner: unsafe fn() -> Option<&'static UnsafeCell<Option<T>>>,

    // initialization routine to invoke to create a value
    init: fn() -> T,
}

#[stable(feature = "std_debug", since = "1.16.0")]
impl<T: 'static> fmt::Debug for LocalKey<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("LocalKey { .. }")
    }
}

/// Declare a new thread local storage key of type [`std::thread::LocalKey`].
///
/// # Syntax
///
/// The macro wraps any number of static declarations and makes them thread local.
/// Publicity and attributes for each static are allowed. Example:
///
/// ```
/// use std::cell::RefCell;
/// thread_local! {
///     pub static FOO: RefCell<u32> = RefCell::new(1);
///
///     #[allow(unused)]
///     static BAR: RefCell<f32> = RefCell::new(1.0);
/// }
/// # fn main() {}
/// ```
///
/// See [LocalKey documentation][`std::thread::LocalKey`] for more
/// information.
///
/// [`std::thread::LocalKey`]: ../std/thread/struct.LocalKey.html
#[macro_export]
#[stable(feature = "rust1", since = "1.0.0")]
#[allow_internal_unstable]
macro_rules! thread_local {
    // empty (base case for the recursion)
    () => {};

    // process multiple declarations
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr; $($rest:tt)*) => (
        $crate::__thread_local_inner!($(#[$attr])* $vis $name, $t, $init);
        $crate::thread_local!($($rest)*);
    );

    // handle a single declaration
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr) => (
        $crate::__thread_local_inner!($(#[$attr])* $vis $name, $t, $init);
    );
}

#[doc(hidden)]
#[unstable(feature = "thread_local_internals",
           reason = "should not be necessary",
           issue = "0")]
#[macro_export]
#[allow_internal_unstable]
#[allow_internal_unsafe]
macro_rules! __thread_local_inner {
    (@key $(#[$attr:meta])* $vis:vis $name:ident, $t:ty, $init:expr) => {
        {
            #[inline]
            fn __init() -> $t { $init }

            unsafe fn __getit() -> $crate::option::Option<
                &'static $crate::cell::UnsafeCell<
                    $crate::option::Option<$t>>>
            {
                #[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
                static __KEY: $crate::thread::__StaticLocalKeyInner<$t> =
                    $crate::thread::__StaticLocalKeyInner::new();

                #[thread_local]
                #[cfg(all(
                    target_thread_local,
                    not(all(target_arch = "wasm32", not(target_feature = "atomics"))),
                ))]
                static __KEY: $crate::thread::__FastLocalKeyInner<$t> =
                    $crate::thread::__FastLocalKeyInner::new();

                #[cfg(all(
                    not(target_thread_local),
                    not(all(target_arch = "wasm32", not(target_feature = "atomics"))),
                ))]
                static __KEY: $crate::thread::__OsLocalKeyInner<$t> =
                    $crate::thread::__OsLocalKeyInner::new();

                __KEY.get()
            }

            unsafe {
                $crate::thread::LocalKey::new(__getit, __init)
            }
        }
    };
    ($(#[$attr:meta])* $vis:vis $name:ident, $t:ty, $init:expr) => {
        $(#[$attr])* $vis const $name: $crate::thread::LocalKey<$t> =
            $crate::__thread_local_inner!(@key $(#[$attr])* $vis $name, $t, $init);
    }
}

/// An error returned by [`LocalKey::try_with`](struct.LocalKey.html#method.try_with).
#[stable(feature = "thread_local_try_with", since = "1.26.0")]
pub struct AccessError {
    _private: (),
}

#[stable(feature = "thread_local_try_with", since = "1.26.0")]
impl fmt::Debug for AccessError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("AccessError").finish()
    }
}

#[stable(feature = "thread_local_try_with", since = "1.26.0")]
impl fmt::Display for AccessError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt("already destroyed", f)
    }
}

impl<T: 'static> LocalKey<T> {
    #[doc(hidden)]
    #[unstable(feature = "thread_local_internals",
               reason = "recently added to create a key",
               issue = "0")]
    pub const unsafe fn new(inner: unsafe fn() -> Option<&'static UnsafeCell<Option<T>>>,
                            init: fn() -> T) -> LocalKey<T> {
        LocalKey {
            inner,
            init,
        }
    }

    /// Acquires a reference to the value in this TLS key.
    ///
    /// This will lazily initialize the value if this thread has not referenced
    /// this key yet.
    ///
    /// # Panics
    ///
    /// This function will `panic!()` if the key currently has its
    /// destructor running, and it **may** panic if the destructor has
    /// previously been run for this thread.
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn with<F, R>(&'static self, f: F) -> R
                      where F: FnOnce(&T) -> R {
        self.try_with(f).expect("cannot access a TLS value during or \
                                 after it is destroyed")
    }

    unsafe fn init(&self, slot: &UnsafeCell<Option<T>>) -> &T {
        // Execute the initialization up front, *then* move it into our slot,
        // just in case initialization fails.
        let value = (self.init)();
        let ptr = slot.get();

        // note that this can in theory just be `*ptr = Some(value)`, but due to
        // the compiler will currently codegen that pattern with something like:
        //
        //      ptr::drop_in_place(ptr)
        //      ptr::write(ptr, Some(value))
        //
        // Due to this pattern it's possible for the destructor of the value in
        // `ptr` (e.g., if this is being recursively initialized) to re-access
        // TLS, in which case there will be a `&` and `&mut` pointer to the same
        // value (an aliasing violation). To avoid setting the "I'm running a
        // destructor" flag we just use `mem::replace` which should sequence the
        // operations a little differently and make this safe to call.
        mem::replace(&mut *ptr, Some(value));

        // After storing `Some` we want to get a reference to the contents of
        // what we just stored. While we could use `unwrap` here and it should
        // always work it empirically doesn't seem to always get optimized away,
        // which means that using something like `try_with` can pull in
        // panicking code and cause a large size bloat.
        match *ptr {
            Some(ref x) => x,
            None => hint::unreachable_unchecked(),
        }
    }

    /// Acquires a reference to the value in this TLS key.
    ///
    /// This will lazily initialize the value if this thread has not referenced
    /// this key yet. If the key has been destroyed (which may happen if this is called
    /// in a destructor), this function will return an [`AccessError`](struct.AccessError.html).
    ///
    /// # Panics
    ///
    /// This function will still `panic!()` if the key is uninitialized and the
    /// key's initializer panics.
    #[stable(feature = "thread_local_try_with", since = "1.26.0")]
    pub fn try_with<F, R>(&'static self, f: F) -> Result<R, AccessError>
    where
        F: FnOnce(&T) -> R,
    {
        unsafe {
            let slot = (self.inner)().ok_or(AccessError {
                _private: (),
            })?;
            Ok(f(match *slot.get() {
                Some(ref inner) => inner,
                None => self.init(slot),
            }))
        }
    }
}

/// On some platforms like wasm32 there's no threads, so no need to generate
/// thread locals and we can instead just use plain statics!
#[doc(hidden)]
#[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
pub mod statik {
    use cell::UnsafeCell;
    use fmt;

    pub struct Key<T> {
        inner: UnsafeCell<Option<T>>,
    }

    unsafe impl<T> ::marker::Sync for Key<T> { }

    impl<T> fmt::Debug for Key<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.pad("Key { .. }")
        }
    }

    impl<T> Key<T> {
        pub const fn new() -> Key<T> {
            Key {
                inner: UnsafeCell::new(None),
            }
        }

        pub unsafe fn get(&self) -> Option<&'static UnsafeCell<Option<T>>> {
            Some(&*(&self.inner as *const _))
        }
    }
}

#[doc(hidden)]
#[cfg(target_thread_local)]
pub mod fast {
    use cell::{Cell, UnsafeCell};
    use fmt;
    use mem;
    use ptr;
    use sys::fast_thread_local::{register_dtor, requires_move_before_drop};

    pub struct Key<T> {
        inner: UnsafeCell<Option<T>>,

        // Metadata to keep track of the state of the destructor. Remember that
        // these variables are thread-local, not global.
        dtor_registered: Cell<bool>,
        dtor_running: Cell<bool>,
    }

    impl<T> fmt::Debug for Key<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.pad("Key { .. }")
        }
    }

    impl<T> Key<T> {
        pub const fn new() -> Key<T> {
            Key {
                inner: UnsafeCell::new(None),
                dtor_registered: Cell::new(false),
                dtor_running: Cell::new(false)
            }
        }

        pub unsafe fn get(&self) -> Option<&'static UnsafeCell<Option<T>>> {
            if mem::needs_drop::<T>() && self.dtor_running.get() {
                return None
            }
            self.register_dtor();
            Some(&*(&self.inner as *const _))
        }

        unsafe fn register_dtor(&self) {
            if !mem::needs_drop::<T>() || self.dtor_registered.get() {
                return
            }

            register_dtor(self as *const _ as *mut u8,
                          destroy_value::<T>);
            self.dtor_registered.set(true);
        }
    }

    unsafe extern fn destroy_value<T>(ptr: *mut u8) {
        let ptr = ptr as *mut Key<T>;
        // Right before we run the user destructor be sure to flag the
        // destructor as running for this thread so calls to `get` will return
        // `None`.
        (*ptr).dtor_running.set(true);

        // Some implementations may require us to move the value before we drop
        // it as it could get re-initialized in-place during destruction.
        //
        // Hence, we use `ptr::read` on those platforms (to move to a "safe"
        // location) instead of drop_in_place.
        if requires_move_before_drop() {
            ptr::read((*ptr).inner.get());
        } else {
            ptr::drop_in_place((*ptr).inner.get());
        }
    }
}

#[doc(hidden)]
pub mod os {
    use cell::{Cell, UnsafeCell};
    use fmt;
    use marker;
    use ptr;
    use sys_common::thread_local::StaticKey as OsStaticKey;

    pub struct Key<T> {
        // OS-TLS key that we'll use to key off.
        os: OsStaticKey,
        marker: marker::PhantomData<Cell<T>>,
    }

    impl<T> fmt::Debug for Key<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.pad("Key { .. }")
        }
    }

    unsafe impl<T> ::marker::Sync for Key<T> { }

    struct Value<T: 'static> {
        key: &'static Key<T>,
        value: UnsafeCell<Option<T>>,
    }

    impl<T: 'static> Key<T> {
        pub const fn new() -> Key<T> {
            Key {
                os: OsStaticKey::new(Some(destroy_value::<T>)),
                marker: marker::PhantomData
            }
        }

        pub unsafe fn get(&'static self) -> Option<&'static UnsafeCell<Option<T>>> {
            let ptr = self.os.get() as *mut Value<T>;
            if !ptr.is_null() {
                if ptr as usize == 1 {
                    return None
                }
                return Some(&(*ptr).value);
            }

            // If the lookup returned null, we haven't initialized our own
            // local copy, so do that now.
            let ptr: Box<Value<T>> = box Value {
                key: self,
                value: UnsafeCell::new(None),
            };
            let ptr = Box::into_raw(ptr);
            self.os.set(ptr as *mut u8);
            Some(&(*ptr).value)
        }
    }

    unsafe extern fn destroy_value<T: 'static>(ptr: *mut u8) {
        // The OS TLS ensures that this key contains a NULL value when this
        // destructor starts to run. We set it back to a sentinel value of 1 to
        // ensure that any future calls to `get` for this thread will return
        // `None`.
        //
        // Note that to prevent an infinite loop we reset it back to null right
        // before we return from the destructor ourselves.
        let ptr = Box::from_raw(ptr as *mut Value<T>);
        let key = ptr.key;
        key.os.set(1 as *mut u8);
        drop(ptr);
        key.os.set(ptr::null_mut());
    }
}

#[cfg(all(test, not(target_os = "emscripten")))]
mod tests {
    use sync::mpsc::{channel, Sender};
    use cell::{Cell, UnsafeCell};
    use thread;

    struct Foo(Sender<()>);

    impl Drop for Foo {
        fn drop(&mut self) {
            let Foo(ref s) = *self;
            s.send(()).unwrap();
        }
    }

    #[test]
    fn smoke_no_dtor() {
        thread_local!(static FOO: Cell<i32> = Cell::new(1));

        FOO.with(|f| {
            assert_eq!(f.get(), 1);
            f.set(2);
        });
        let (tx, rx) = channel();
        let _t = thread::spawn(move|| {
            FOO.with(|f| {
                assert_eq!(f.get(), 1);
            });
            tx.send(()).unwrap();
        });
        rx.recv().unwrap();

        FOO.with(|f| {
            assert_eq!(f.get(), 2);
        });
    }

    #[test]
    fn states() {
        struct Foo;
        impl Drop for Foo {
            fn drop(&mut self) {
                assert!(FOO.try_with(|_| ()).is_err());
            }
        }
        thread_local!(static FOO: Foo = Foo);

        thread::spawn(|| {
            assert!(FOO.try_with(|_| ()).is_ok());
        }).join().ok().unwrap();
    }

    #[test]
    fn smoke_dtor() {
        thread_local!(static FOO: UnsafeCell<Option<Foo>> = UnsafeCell::new(None));

        let (tx, rx) = channel();
        let _t = thread::spawn(move|| unsafe {
            let mut tx = Some(tx);
            FOO.with(|f| {
                *f.get() = Some(Foo(tx.take().unwrap()));
            });
        });
        rx.recv().unwrap();
    }

    #[test]
    fn circular() {
        struct S1;
        struct S2;
        thread_local!(static K1: UnsafeCell<Option<S1>> = UnsafeCell::new(None));
        thread_local!(static K2: UnsafeCell<Option<S2>> = UnsafeCell::new(None));
        static mut HITS: u32 = 0;

        impl Drop for S1 {
            fn drop(&mut self) {
                unsafe {
                    HITS += 1;
                    if K2.try_with(|_| ()).is_err() {
                        assert_eq!(HITS, 3);
                    } else {
                        if HITS == 1 {
                            K2.with(|s| *s.get() = Some(S2));
                        } else {
                            assert_eq!(HITS, 3);
                        }
                    }
                }
            }
        }
        impl Drop for S2 {
            fn drop(&mut self) {
                unsafe {
                    HITS += 1;
                    assert!(K1.try_with(|_| ()).is_ok());
                    assert_eq!(HITS, 2);
                    K1.with(|s| *s.get() = Some(S1));
                }
            }
        }

        thread::spawn(move|| {
            drop(S1);
        }).join().ok().unwrap();
    }

    #[test]
    fn self_referential() {
        struct S1;
        thread_local!(static K1: UnsafeCell<Option<S1>> = UnsafeCell::new(None));

        impl Drop for S1 {
            fn drop(&mut self) {
                assert!(K1.try_with(|_| ()).is_err());
            }
        }

        thread::spawn(move|| unsafe {
            K1.with(|s| *s.get() = Some(S1));
        }).join().ok().unwrap();
    }

    // Note that this test will deadlock if TLS destructors aren't run (this
    // requires the destructor to be run to pass the test). macOS has a known bug
    // where dtors-in-dtors may cancel other destructors, so we just ignore this
    // test on macOS.
    #[test]
    #[cfg_attr(target_os = "macos", ignore)]
    fn dtors_in_dtors_in_dtors() {
        struct S1(Sender<()>);
        thread_local!(static K1: UnsafeCell<Option<S1>> = UnsafeCell::new(None));
        thread_local!(static K2: UnsafeCell<Option<Foo>> = UnsafeCell::new(None));

        impl Drop for S1 {
            fn drop(&mut self) {
                let S1(ref tx) = *self;
                unsafe {
                    let _ = K2.try_with(|s| *s.get() = Some(Foo(tx.clone())));
                }
            }
        }

        let (tx, rx) = channel();
        let _t = thread::spawn(move|| unsafe {
            let mut tx = Some(tx);
            K1.with(|s| *s.get() = Some(S1(tx.take().unwrap())));
        });
        rx.recv().unwrap();
    }
}

#[cfg(test)]
mod dynamic_tests {
    use cell::RefCell;
    use collections::HashMap;

    #[test]
    fn smoke() {
        fn square(i: i32) -> i32 { i * i }
        thread_local!(static FOO: i32 = square(3));

        FOO.with(|f| {
            assert_eq!(*f, 9);
        });
    }

    #[test]
    fn hashmap() {
        fn map() -> RefCell<HashMap<i32, i32>> {
            let mut m = HashMap::new();
            m.insert(1, 2);
            RefCell::new(m)
        }
        thread_local!(static FOO: RefCell<HashMap<i32, i32>> = map());

        FOO.with(|map| {
            assert_eq!(map.borrow()[&1], 2);
        });
    }

    #[test]
    fn refcell_vec() {
        thread_local!(static FOO: RefCell<Vec<u32>> = RefCell::new(vec![1, 2, 3]));

        FOO.with(|vec| {
            assert_eq!(vec.borrow().len(), 3);
            vec.borrow_mut().push(4);
            assert_eq!(vec.borrow()[3], 4);
        });
    }
}
