#![allow(dead_code)]

use better_default::Default;

#[derive(Default, Debug)]
enum Enum {
    // mark the variant as default, and also specifies the default values :
    //      - the first field keeps it's usual default value.
    //      - the second field (which is at index 1) will have it's default value set to "Hello world!"
    #[default(1: "Hello world!".to_string())]
    Variant1(u32, String),

    Variant2,

    Variant3,
}

fn main() {
    let default = Enum::default();
    
    // should print "Variant1(1, "Hello world!")"
    println!("{:?}", default);
}
