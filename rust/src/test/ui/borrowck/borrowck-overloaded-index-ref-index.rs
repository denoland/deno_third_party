// revisions: ast mir
//[mir]compile-flags: -Z borrowck=mir

use std::ops::{Index, IndexMut};

struct Foo {
    x: isize,
    y: isize,
}

impl<'a> Index<&'a String> for Foo {
    type Output = isize;

    fn index(&self, z: &String) -> &isize {
        if *z == "x" {
            &self.x
        } else {
            &self.y
        }
    }
}

impl<'a> IndexMut<&'a String> for Foo {
    fn index_mut(&mut self, z: &String) -> &mut isize {
        if *z == "x" {
            &mut self.x
        } else {
            &mut self.y
        }
    }
}

struct Bar {
    x: isize,
}

impl Index<isize> for Bar {
    type Output = isize;

    fn index<'a>(&'a self, z: isize) -> &'a isize {
        &self.x
    }
}

fn main() {
    let mut f = Foo {
        x: 1,
        y: 2,
    };
    let mut s = "hello".to_string();
    let rs = &mut s;
    println!("{}", f[&s]);
    //[ast]~^ ERROR cannot borrow `s` as immutable because it is also borrowed as mutable
    //[mir]~^^ ERROR cannot borrow `s` as immutable because it is also borrowed as mutable
    f[&s] = 10;
    //[ast]~^ ERROR cannot borrow `s` as immutable because it is also borrowed as mutable
    //[mir]~^^ ERROR cannot borrow `s` as immutable because it is also borrowed as mutable
    let s = Bar {
        x: 1,
    };
    s[2] = 20;
    //[ast]~^ ERROR cannot assign to immutable indexed content
    //[mir]~^^ ERROR cannot assign to data in a `&` reference
    drop(rs);
}
