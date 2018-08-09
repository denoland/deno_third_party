// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// ignore-tidy-linelength
// revisions: ast mir
//[mir]compile-flags: -Z borrowck=mir

#![feature(slice_patterns)]

pub struct Foo {
  x: u32
}

pub struct Bar(u32);

pub enum Baz {
    X(u32)
}

union U {
    a: u8,
    b: u64,
}

impl Foo {
  fn x(&mut self) -> &mut u32 { &mut self.x }
}

impl Bar {
    fn x(&mut self) -> &mut u32 { &mut self.0 }
}

impl Baz {
    fn x(&mut self) -> &mut u32 {
        match *self {
            Baz::X(ref mut value) => value
        }
    }
}

fn main() {
    // Local and field from struct
    {
        let mut f = Foo { x: 22 };
        let x = f.x();
        f.x; //[ast]~ ERROR cannot use `f.x` because it was mutably borrowed
        //[mir]~^ ERROR cannot use `f.x` because it was mutably borrowed
        drop(x);
    }
    // Local and field from tuple-struct
    {
        let mut g = Bar(22);
        let x = g.x();
        g.0; //[ast]~ ERROR cannot use `g.0` because it was mutably borrowed
             //[mir]~^ ERROR cannot use `g.0` because it was mutably borrowed
        drop(x);
    }
    // Local and field from tuple
    {
        let mut h = (22, 23);
        let x = &mut h.0;
        h.0; //[ast]~ ERROR cannot use `h.0` because it was mutably borrowed
             //[mir]~^ ERROR cannot use `h.0` because it was mutably borrowed
        drop(x);
    }
    // Local and field from enum
    {
        let mut e = Baz::X(2);
        let x = e.x();
        match e { //[mir]~ ERROR cannot use `e` because it was mutably borrowed
            Baz::X(value) => value
            //[ast]~^ ERROR cannot use `e.0` because it was mutably borrowed
            //[mir]~^^ ERROR cannot use `e.0` because it was mutably borrowed
        };
        drop(x);
    }
    // Local and field from union
    unsafe {
        let mut u = U { b: 0 };
        let x = &mut u.a;
        u.a; //[ast]~ ERROR cannot use `u.a` because it was mutably borrowed
             //[mir]~^ ERROR cannot use `u.a` because it was mutably borrowed
        drop(x);
    }
    // Deref and field from struct
    {
        let mut f = Box::new(Foo { x: 22 });
        let x = f.x();
        f.x; //[ast]~ ERROR cannot use `f.x` because it was mutably borrowed
             //[mir]~^ ERROR cannot use `f.x` because it was mutably borrowed
        drop(x);
    }
    // Deref and field from tuple-struct
    {
        let mut g = Box::new(Bar(22));
        let x = g.x();
        g.0; //[ast]~ ERROR cannot use `g.0` because it was mutably borrowed
             //[mir]~^ ERROR cannot use `g.0` because it was mutably borrowed
        drop(x);
    }
    // Deref and field from tuple
    {
        let mut h = Box::new((22, 23));
        let x = &mut h.0;
        h.0; //[ast]~ ERROR cannot use `h.0` because it was mutably borrowed
             //[mir]~^ ERROR cannot use `h.0` because it was mutably borrowed
        drop(x);
    }
    // Deref and field from enum
    {
        let mut e = Box::new(Baz::X(3));
        let x = e.x();
        match *e { //[mir]~ ERROR cannot use `*e` because it was mutably borrowed
            Baz::X(value) => value
            //[ast]~^ ERROR cannot use `e.0` because it was mutably borrowed
            //[mir]~^^ ERROR cannot use `e.0` because it was mutably borrowed
        };
        drop(x);
    }
    // Deref and field from union
    unsafe {
        let mut u = Box::new(U { b: 0 });
        let x = &mut u.a;
        u.a; //[ast]~ ERROR cannot use `u.a` because it was mutably borrowed
             //[mir]~^ ERROR cannot use `u.a` because it was mutably borrowed
        drop(x);
    }
    // Constant index
    {
        let mut v = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let x = &mut v;
        match v { //[mir]~ ERROR cannot use `v` because it was mutably borrowed
            &[x, _, .., _, _] => println!("{}", x),
                //[ast]~^ ERROR cannot use `v[..]` because it was mutably borrowed
                //[mir]~^^ ERROR cannot use `v[..]` because it was mutably borrowed
                            _ => panic!("other case"),
        }
        match v { //[mir]~ ERROR cannot use `v` because it was mutably borrowed
            &[_, x, .., _, _] => println!("{}", x),
                //[ast]~^ ERROR cannot use `v[..]` because it was mutably borrowed
                //[mir]~^^ ERROR cannot use `v[..]` because it was mutably borrowed
                            _ => panic!("other case"),
        }
        match v { //[mir]~ ERROR cannot use `v` because it was mutably borrowed
            &[_, _, .., x, _] => println!("{}", x),
                //[ast]~^ ERROR cannot use `v[..]` because it was mutably borrowed
                //[mir]~^^ ERROR cannot use `v[..]` because it was mutably borrowed
                            _ => panic!("other case"),
        }
        match v { //[mir]~ ERROR cannot use `v` because it was mutably borrowed
            &[_, _, .., _, x] => println!("{}", x),
                //[ast]~^ ERROR cannot use `v[..]` because it was mutably borrowed
                //[mir]~^^ ERROR cannot use `v[..]` because it was mutably borrowed
                            _ => panic!("other case"),
        }
        drop(x);
    }
    // Subslices
    {
        let mut v = &[1, 2, 3, 4, 5];
        let x = &mut v;
        match v { //[mir]~ ERROR cannot use `v` because it was mutably borrowed
            &[x..] => println!("{:?}", x),
                //[ast]~^ ERROR cannot use `v[..]` because it was mutably borrowed
                //[mir]~^^ ERROR cannot use `v[..]` because it was mutably borrowed
            _ => panic!("other case"),
        }
        match v { //[mir]~ ERROR cannot use `v` because it was mutably borrowed
            &[_, x..] => println!("{:?}", x),
                //[ast]~^ ERROR cannot use `v[..]` because it was mutably borrowed
                //[mir]~^^ ERROR cannot use `v[..]` because it was mutably borrowed
            _ => panic!("other case"),
        }
        match v { //[mir]~ ERROR cannot use `v` because it was mutably borrowed
            &[x.., _] => println!("{:?}", x),
                //[ast]~^ ERROR cannot use `v[..]` because it was mutably borrowed
                //[mir]~^^ ERROR cannot use `v[..]` because it was mutably borrowed
            _ => panic!("other case"),
        }
        match v { //[mir]~ ERROR cannot use `v` because it was mutably borrowed
            &[_, x.., _] => println!("{:?}", x),
                //[ast]~^ ERROR cannot use `v[..]` because it was mutably borrowed
                //[mir]~^^ ERROR cannot use `v[..]` because it was mutably borrowed
            _ => panic!("other case"),
        }
        drop(x);
    }
    // Downcasted field
    {
        enum E<X> { A(X), B { x: X } }

        let mut e = E::A(3);
        let x = &mut e;
        match e { //[mir]~ ERROR cannot use `e` because it was mutably borrowed
            E::A(ref ax) =>
                //[ast]~^ ERROR cannot borrow `e.0` as immutable because `e` is also borrowed as mutable
                //[mir]~^^ ERROR cannot borrow `e.0` as immutable because it is also borrowed as mutable
                //[mir]~| ERROR cannot use `e` because it was mutably borrowed
                println!("e.ax: {:?}", ax),
            E::B { x: ref bx } =>
                //[ast]~^ ERROR cannot borrow `e.x` as immutable because `e` is also borrowed as mutable
                //[mir]~^^ ERROR cannot borrow `e.x` as immutable because it is also borrowed as mutable
                println!("e.bx: {:?}", bx),
        }
        drop(x);
    }
    // Field in field
    {
        struct F { x: u32, y: u32 };
        struct S { x: F, y: (u32, u32), };
        let mut s = S { x: F { x: 1, y: 2}, y: (999, 998) };
        let x = &mut s;
        match s { //[mir]~ ERROR cannot use `s` because it was mutably borrowed
            S  { y: (ref y0, _), .. } =>
                //[ast]~^ ERROR cannot borrow `s.y.0` as immutable because `s` is also borrowed as mutable
                //[mir]~^^ ERROR cannot borrow `s.y.0` as immutable because it is also borrowed as mutable
                println!("y0: {:?}", y0),
            _ => panic!("other case"),
        }
        match s { //[mir]~ ERROR cannot use `s` because it was mutably borrowed
            S  { x: F { y: ref x0, .. }, .. } =>
                //[ast]~^ ERROR cannot borrow `s.x.y` as immutable because `s` is also borrowed as mutable
                //[mir]~^^ ERROR cannot borrow `s.x.y` as immutable because it is also borrowed as mutable
                println!("x0: {:?}", x0),
            _ => panic!("other case"),
        }
        drop(x);
    }
    // Field of ref
    {
        struct Block<'a> {
            current: &'a u8,
            unrelated: &'a u8,
        };

        fn bump<'a>(mut block: &mut Block<'a>) {
            let x = &mut block;
            let p: &'a u8 = &*block.current;
            //[mir]~^ ERROR cannot borrow `*block.current` as immutable because it is also borrowed as mutable
            // No errors in AST because of issue rust#38899
            drop(x);
        }
    }
    // Field of ptr
    {
        struct Block2 {
            current: *const u8,
            unrelated: *const u8,
        }

        unsafe fn bump2(mut block: *mut Block2) {
            let x = &mut block;
            let p : *const u8 = &*(*block).current;
            //[mir]~^ ERROR cannot borrow `*block.current` as immutable because it is also borrowed as mutable
            // No errors in AST because of issue rust#38899
            drop(x);
        }
    }
    // Field of index
    {
        struct F {x: u32, y: u32};
        let mut v = &[F{x: 1, y: 2}, F{x: 3, y: 4}];
        let x = &mut v;
        v[0].y;
        //[ast]~^ ERROR cannot use `v[..].y` because it was mutably borrowed
        //[mir]~^^ ERROR cannot use `v[..].y` because it was mutably borrowed
        //[mir]~| ERROR cannot use `*v` because it was mutably borrowed
        drop(x);
    }
    // Field of constant index
    {
        struct F {x: u32, y: u32};
        let mut v = &[F{x: 1, y: 2}, F{x: 3, y: 4}];
        let x = &mut v;
        match v { //[mir]~ ERROR cannot use `v` because it was mutably borrowed
            &[_, F {x: ref xf, ..}] => println!("{}", xf),
            //[mir]~^ ERROR cannot borrow `v[..].x` as immutable because it is also borrowed as mutable
            // No errors in AST
            _ => panic!("other case")
        }
        drop(x);
    }
    // Field from upvar
    {
        let mut x = 0;
        || {
            let y = &mut x;
            &mut x; //[ast]~ ERROR cannot borrow `**x` as mutable more than once at a time
                    //[mir]~^ ERROR cannot borrow `x` as mutable more than once at a time
            *y = 1;
        };
    }
    // Field from upvar nested
    {
        // FIXME(#49824) -- the free region error below should probably not be there
        let mut x = 0;
           || {
               || { //[mir]~ ERROR free region `` does not outlive
                   let y = &mut x;
                   &mut x; //[ast]~ ERROR cannot borrow `**x` as mutable more than once at a time
                   //[mir]~^ ERROR cannot borrow `x` as mutable more than once at a time
                   *y = 1;
                   drop(y);
                }
           };
    }
    {
        fn foo(x: Vec<i32>) {
            let c = || {
                drop(x);
                drop(x); //[ast]~ ERROR use of moved value: `x`
                         //[mir]~^ ERROR use of moved value: `x`
            };
            c();
        }
    }
}
