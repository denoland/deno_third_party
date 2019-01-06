#![allow(unused_mut)]
#![feature(generators, generator_trait)]

use std::ops::Generator;
use std::ops::GeneratorState::Yielded;

pub struct GenIter<G>(G);

impl <G> Iterator for GenIter<G>
where
    G: Generator,
{
    type Item = G::Yield;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            match self.0.resume() {
                Yielded(y) => Some(y),
                _ => None
            }
        }
    }
}

fn bug<'a>() -> impl Iterator<Item = &'a str> {
    GenIter(move || {
        let mut s = String::new();
        yield &s[..] //~ ERROR `s` does not live long enough [E0597]
    })
}

fn main() {
    bug();
}
