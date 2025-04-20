#![allow(dead_code)]

use better_default::Default;

#[derive(Default, Debug)]
// here we can use the top default attribute to customize the default values of our fields.
//      - we change the default value of the first field (represented by the index 0) to 1
#[default(0: 1)]
struct Struct(u32, String);

fn main() {
    let default = Struct::default();
    println!("{:?}", default) // should print "Struct(1, "")"
}
