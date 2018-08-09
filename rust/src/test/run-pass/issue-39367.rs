// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::ops::Deref;

struct ArenaSet<U: Deref, V=<U as Deref>::Target>(U, &'static V)
    where V: 'static + ?Sized;

static Z: [u8; 4] = [1,2,3,4];

fn arena() -> &'static ArenaSet<Vec<u8>> {
    fn __static_ref_initialize() -> ArenaSet<Vec<u8>> {
        ArenaSet(vec![], &Z)
    }
    unsafe {
        use std::sync::{Once, ONCE_INIT};
        fn require_sync<T: Sync>(_: &T) { }
        unsafe fn __stability() -> &'static ArenaSet<Vec<u8>> {
            use std::mem::transmute;
            use std::boxed::Box;
            static mut DATA: *const ArenaSet<Vec<u8>> = 0 as *const ArenaSet<Vec<u8>>;

            static mut ONCE: Once = ONCE_INIT;
            ONCE.call_once(|| {
                DATA = transmute
                    ::<Box<ArenaSet<Vec<u8>>>, *const ArenaSet<Vec<u8>>>
                    (Box::new(__static_ref_initialize()));
            });

            &*DATA
        }
        let static_ref = __stability();
        require_sync(static_ref);
        static_ref
    }
}

fn main() {
    let &ArenaSet(ref u, v) = arena();
    assert!(u.is_empty());
    assert_eq!(v, Z);
}
