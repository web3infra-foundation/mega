#![allow(dead_code)]

use better_default::Default;

#[derive(Default, Eq, PartialEq, Debug)]
enum Enum2 {
    #[default]
    Variant {
        first: u32,
        second: String,
    },

    Variant2,

    Variant3,
}

fn main() {
    assert_eq!(Enum2::Variant { first: 0, second: String::new() }, Enum2::default())
}
