#![feature(existential_type)]

fn main() {
}

existential type PartiallyDefined<T>: 'static; //~ `T` is unused

fn partially_defined<T: std::fmt::Debug>(_: T) -> PartiallyDefined<T> {
    4u32
}

existential type PartiallyDefined2<T>: 'static; //~ `T` is unused

fn partially_defined2<T: std::fmt::Debug>(_: T) -> PartiallyDefined2<T> {
    4u32
}

fn partially_defined22<T>(_: T) -> PartiallyDefined2<T> {
    4u32
}
