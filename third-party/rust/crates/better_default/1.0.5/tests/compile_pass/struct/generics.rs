use better_default::Default;

#[derive(Default)]
struct Struct<T: Default> {
    field: T,
    field2: String
}

fn main() {
    
}
