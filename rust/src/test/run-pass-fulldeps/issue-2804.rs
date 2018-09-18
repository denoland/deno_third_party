// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
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

use std::collections::HashMap;
use serialize::json::{self, Json};
use std::option;

enum object {
    bool_value(bool),
    int_value(i64),
}

fn lookup(table: json::Object, key: String, default: String) -> String
{
    match table.get(&key) {
        option::Option::Some(&Json::String(ref s)) => {
            s.to_string()
        }
        option::Option::Some(value) => {
            println!("{} was expected to be a string but is a {}", key, value);
            default
        }
        option::Option::None => {
            default
        }
    }
}

fn add_interface(_store: isize, managed_ip: String, data: json::Json) -> (String, object)
{
    match &data {
        &Json::Object(ref interface) => {
            let name = lookup(interface.clone(),
                              "ifDescr".to_string(),
                              "".to_string());
            let label = format!("{}-{}", managed_ip, name);

            (label, object::bool_value(false))
        }
        _ => {
            println!("Expected dict for {} interfaces, found {}", managed_ip, data);
            ("gnos:missing-interface".to_string(), object::bool_value(true))
        }
    }
}

fn add_interfaces(store: isize, managed_ip: String, device: HashMap<String, json::Json>)
-> Vec<(String, object)> {
    match device["interfaces"] {
        Json::Array(ref interfaces) =>
        {
          interfaces.iter().map(|interface| {
                add_interface(store, managed_ip.clone(), (*interface).clone())
          }).collect()
        }
        _ =>
        {
            println!("Expected list for {} interfaces, found {}", managed_ip,
                     device["interfaces"]);
            Vec::new()
        }
    }
}

pub fn main() {}
