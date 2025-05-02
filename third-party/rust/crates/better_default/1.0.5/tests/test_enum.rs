#![allow(dead_code)]

use better_default::Default;

#[derive(Default, PartialEq, Debug)]
enum Enum1 {
    Variant1 {},

    #[default]
    Variant2,
}

#[test]
fn test_unit_variant() {
    assert_eq!(Enum1::default(), Enum1::Variant2)
}

#[derive(Default, PartialEq, Debug)]
enum Enum2 {
    #[default(field1: 1.0, field2: "aaaa".to_string())]
    Variant1 {
        field1: f32,
        field2: String,
    },

    Variant2,
}

#[test]
fn test_named_variant_top_attribute() {
    let default = Enum2::default();
    let expected = Enum2::Variant1 {
        field1: 1.0,
        field2: "aaaa".to_string(),
    };

    assert_eq!(default, expected);
}

#[derive(Default, PartialEq, Debug)]
enum Enum3 {
    #[default]
    Variant1 {
        #[default(1.0)]
        field1: f32,

        #[default("aaaa".to_string())]
        field2: String,
    },

    Variant2,
}

#[test]
fn test_named_variant_inner_attributes() {
    let default = Enum3::default();
    let expected = Enum3::Variant1 {
        field1: 1.0,
        field2: "aaaa".to_string(),
    };

    assert_eq!(default, expected);
}

#[derive(Default, PartialEq, Debug)]
enum Enum4 {
    #[default(0: 1.0, 1: "aaaa".to_string())]
    Variant1(f32, String),

    Variant2,
}

#[test]
fn test_unnamed_variant_top_attribute() {
    let default = Enum4::default();
    let expected = Enum4::Variant1(1.0, "aaaa".to_string());

    assert_eq!(default, expected);
}

#[derive(Default, PartialEq, Debug)]
enum Enum5 {
    #[default]
    Variant1(#[default(1.0)] f32, #[default("aaaa".to_string())] String),

    Variant2,
}

#[test]
fn test_unnamed_variant_inner_attributes() {
    let default = Enum5::default();
    let expected = Enum5::Variant1(1.0, "aaaa".to_string());

    assert_eq!(default, expected);
}
