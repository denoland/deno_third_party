// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

fn id<T>(x: T) -> T { x }

struct StateMachineIter<'a> {
    statefn: &'a StateMachineFunc<'a>
}

type StateMachineFunc<'a> = fn(&mut StateMachineIter<'a>) -> Option<&'static str>;

impl<'a> Iterator for StateMachineIter<'a> {
    type Item = &'static str;

    fn next(&mut self) -> Option<&'static str> {
        return  (*self.statefn)(self);
    }
}

fn state1(self_: &mut StateMachineIter) -> Option<&'static str> {
    self_.statefn = &id(state2 as StateMachineFunc);
    //~^ ERROR borrowed value does not live long enough
    return Some("state1");
}

fn state2(self_: &mut StateMachineIter) -> Option<(&'static str)> {
    self_.statefn = &id(state3 as StateMachineFunc);
    //~^ ERROR borrowed value does not live long enough
    return Some("state2");
}

fn state3(self_: &mut StateMachineIter) -> Option<(&'static str)> {
    self_.statefn = &id(finished as StateMachineFunc);
    //~^ ERROR borrowed value does not live long enough
    return Some("state3");
}

fn finished(_: &mut StateMachineIter) -> Option<(&'static str)> {
    return None;
}

fn state_iter() -> StateMachineIter<'static> {
    StateMachineIter {
        statefn: &id(state1 as StateMachineFunc)
        //~^ ERROR borrowed value does not live long enough
    }
}


fn main() {
    let mut it = state_iter();
    println!("{:?}",it.next());
    println!("{:?}",it.next());
    println!("{:?}",it.next());
    println!("{:?}",it.next());
    println!("{:?}",it.next());
}
