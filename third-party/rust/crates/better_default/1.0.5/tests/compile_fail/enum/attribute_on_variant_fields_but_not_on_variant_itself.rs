use better_default::Default;

#[derive(Default)]
enum Enum2 {
    Variant {
        #[default(1)]
        first: u32,
        #[default("aaaaaa".to_string())]
        second: String,
    },

    #[default]
    Variant2,

    Variant3,
}

fn main() {}