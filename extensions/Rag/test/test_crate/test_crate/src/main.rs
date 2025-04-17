pub struct TestStruct {
    pub field: String,
}

pub enum TestEnum {
    Variant1,
    Variant2(String),
}

pub trait TestTrait {
    fn test_method(&self);
}

impl TestTrait for TestStruct {
    fn test_method(&self) {
        println!("Test method called");
    }
}

pub type TestType = String;

pub const TEST_CONST: i32 = 42;

pub static TEST_STATIC: &str = "test";

pub mod test_module {
    pub fn module_function() {
        println!("Module function called");
    }
}

pub fn test_function() {
    println!("Hello, world!");
}

fn main() {
    println!("Hello, world!");
}
