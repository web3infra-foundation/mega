#![allow(dead_code)]

use better_default::Default;

#[derive(Default)]
enum Enum {
    #[default(0: "aaa")]
    Variant(u32, String),

    Variant2,

    Variant3,
}

#[derive(Default)]
enum Enum2 {
    #[default]
    Variant{
        #[default("aaaa")]
        field1: u32,

        field2: String
    },

    Variant2,

    Variant3,
}

#[derive(Default)]
struct Struct {
    #[default("aaaa")]
    field: u32
}

fn main() {}
