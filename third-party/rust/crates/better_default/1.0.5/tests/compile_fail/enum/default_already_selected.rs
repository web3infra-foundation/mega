#![allow(dead_code)]

use better_default::Default;

#[derive(Default)]
enum Enum2 {
    #[default]
    Variant {
        first: u32,
        second: String,
    },

    #[default]
    Variant2,

    Variant3,
}

#[derive(Default)]
enum Enum3 {
    #[default(first: 0)]
    Variant {
        first: u32,
        second: String,
    },

    #[default(0: -1)]
    Variant2(i32),

    Variant3,
}

fn main() {}