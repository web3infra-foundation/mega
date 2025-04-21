#![allow(dead_code)]

use better_default::Default;

#[derive(Default)]
enum Enum<T: Default> {
    #[default]
    Variant(T)
}

#[derive(Default)]
enum Enum2<T: Default> {
    #[default]
    Variant {
        field: T
    }
}

fn main() {}