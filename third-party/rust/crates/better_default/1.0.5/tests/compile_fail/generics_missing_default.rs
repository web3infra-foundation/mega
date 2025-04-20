#![allow(dead_code)]

use better_default::Default;

#[derive(Default)]
enum Enum<T> {
    #[default]
    Variant(T)
}

#[derive(Default)]
enum Enum2<T> {
    #[default]
    Variant {
        field: T
    }
}

#[derive(Default)]
struct Struct<T> {
    field: T
}

fn main() {}