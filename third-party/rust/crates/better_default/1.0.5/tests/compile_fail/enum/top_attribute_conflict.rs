#![allow(dead_code)]

use better_default::Default;

#[derive(Default)]
enum Enum {
    #[default(0: 5)]
    Variant(
        #[default(10)]
        u32, 
        String
    ),

    Variant2,

    Variant3,
}

#[derive(Default)]
enum Enum2 {
    #[default(field1: 10)]
    Variant{
        #[default(5)]
        field1: u32,

        field2: String
    },

    Variant2,

    Variant3,
}

fn main() {}
