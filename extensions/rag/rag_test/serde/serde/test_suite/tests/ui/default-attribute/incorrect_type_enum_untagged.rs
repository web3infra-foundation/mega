// Tests that type error points to the path in attribute

use serde_derive::Deserialize;

#[derive(Deserialize)]
#[serde(untagged)]
enum Enum {
    // Newtype variants do not use the provided path, so it is forbidden here
    // Newtype(#[serde(default = "main")] u8),
    Tuple(u8, #[serde(default = "main")] i8),
    Struct {
        #[serde(default = "main")]
        f1: u8,
        f2: u8,
        #[serde(default = "main")]
        f3: i8,
    },
}

fn main() {}
