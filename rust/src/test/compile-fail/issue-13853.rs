// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

trait Node {
    fn zomg();
}

trait Graph<N: Node> {
    fn nodes<'a, I: Iterator<Item=&'a N>>(&'a self) -> I
        where N: 'a;
}

impl<N: Node> Graph<N> for Vec<N> {
    fn nodes<'a, I: Iterator<Item=&'a N>>(&self) -> I
        where N: 'a
    {
        self.iter() //~ ERROR mismatched types
    }
}

struct Stuff;

impl Node for Stuff {
    fn zomg() {
        println!("zomg");
    }
}

fn iterate<N: Node, G: Graph<N>>(graph: &G) {
    for node in graph.iter() { //~ ERROR no method named `iter` found
        node.zomg();
    }
}

pub fn main() {
    let graph = Vec::new();

    graph.push(Stuff);

    iterate(graph); //~ ERROR mismatched types
}
