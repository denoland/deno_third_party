// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(rustc_private)]

extern crate serialize;

use serialize::{Encodable, Decodable};
use serialize::json;

#[derive(Encodable, Decodable, PartialEq, Debug)]
struct UnitLikeStruct;

pub fn main() {
    let obj = UnitLikeStruct;
    let json_str: String = json::encode(&obj).unwrap();

    let json_object = json::from_str(&json_str);
    let mut decoder = json::Decoder::new(json_object.unwrap());
    let mut decoded_obj: UnitLikeStruct = Decodable::decode(&mut decoder).unwrap();

    assert_eq!(obj, decoded_obj);
}
