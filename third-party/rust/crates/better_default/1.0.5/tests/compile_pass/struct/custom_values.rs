use better_default::Default;

#[derive(Default, Debug, Eq, PartialEq)]
struct Struct {
    #[default(1)]
    field: u32,

    #[default("aaaa".to_string())]
    field2: String
}

fn main() {
    assert_eq!(Struct { field: 1, field2: String::from("aaaa") }, Struct::default())
}

