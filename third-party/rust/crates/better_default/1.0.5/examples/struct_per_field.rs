#![allow(dead_code)]

use better_default::Default;

#[derive(Default, Debug)]
// Top default attributes are optional for structs.
struct Struct {
    #[default(10)] // set the default value of field1 to be 10
    field1: u32,

    field2: String,
}

fn main() {
    let default = Struct::default();
    println!("{:?}", default) // should print "Struct { field1: 10, field2: "" }"
}
