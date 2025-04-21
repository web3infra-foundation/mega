#![allow(dead_code)]

use better_default::Default;

#[derive(Default, Debug)]
enum Enum {
    #[default] // mark the variant as default
    Variant1 {
        #[default(1)] // set the default value of `first` to 1
        first: u32,

        // keep the default value for `second`
        second: String,
    },

    Variant2,

    Variant3,
}

fn main() {
    let default = Enum::default();
    
    // should print "Variant1 { first: 1, second: "" }"
    println!("{:?}", default);
}
