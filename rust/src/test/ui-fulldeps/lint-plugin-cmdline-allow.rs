// run-pass
// aux-build:lint_plugin_test.rs
// ignore-stage1
// compile-flags: -A test-lint

#![feature(plugin)]
#![warn(unused)]
#![plugin(lint_plugin_test)]

fn lintme() { }

pub fn main() {
}
