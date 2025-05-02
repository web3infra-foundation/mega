#![allow(dead_code)]

use better_default::Default;

#[derive(Default)]
enum Enum2 {
    #[default]
    #[default]
    Variant {
        first: u32,
        second: String,
    },

    Variant2,

    Variant3,
}

#[derive(Default)]
struct Struct {
    #[default(0)]
    #[default(1)]
    field: u32
}

fn main() {}