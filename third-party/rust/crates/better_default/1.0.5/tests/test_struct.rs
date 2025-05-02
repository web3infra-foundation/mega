use better_default::Default;

const F4_DEFAULT: [u32; 4] = [1, 2, 3, 4];

fn f3_default() -> &'static str {
    "bbb"
}

#[derive(Default)]
struct Struct<'l> {
    #[default(1)]
    f1: u32,

    #[default("aaa".to_string())]
    f2: String,

    #[default(f3_default())]
    f3: &'l str,

    #[default(F4_DEFAULT)]
    f4: [u32; 4],

    f5: Vec<(f32, char)>,
}

#[test]
fn test_named_fields_per_fields_attrs() {
    let s = Struct::default();

    assert_eq!(s.f1, 1);
    assert_eq!(s.f2, "aaa");
    assert_eq!(s.f3, "bbb");
    assert_eq!(s.f4, [1, 2, 3, 4]);
    assert_eq!(s.f5, vec![]);
}

#[derive(Default)]
#[default(f1: 1, f2: "aaa".to_string(), f3: f3_default(), f4: F4_DEFAULT)]
struct Struct2<'l> {
    f1: u32,

    f2: String,

    f3: &'l str,

    f4: [u32; 4],

    f5: Vec<(f32, char)>,
}

#[test]
fn test_named_fields_top_field_attrs() {
    let s = Struct2::default();

    assert_eq!(s.f1, 1);
    assert_eq!(s.f2, "aaa");
    assert_eq!(s.f3, "bbb");
    assert_eq!(s.f4, [1, 2, 3, 4]);
    assert_eq!(s.f5, vec![]);
}

#[derive(Default)]
struct Struct3<'l>(
    #[default(1)] u32,
    #[default("aaa".to_string())] String,
    #[default(f3_default())] &'l str,
    #[default(F4_DEFAULT)] [u32; 4],
    Vec<(f32, char)>,
);

#[test]
fn test_unnamed_fields_per_fields_attrs() {
    let s = Struct3::default();

    assert_eq!(s.0, 1);
    assert_eq!(s.1, "aaa");
    assert_eq!(s.2, "bbb");
    assert_eq!(s.3, [1, 2, 3, 4]);
    assert_eq!(s.4, vec![]);
}

#[derive(Default)]
#[default(0: 1, 1: "aaa".to_string(), 2: f3_default(), 3: F4_DEFAULT)]
struct Struct4<'l>(u32, String, &'l str, [u32; 4], Vec<(f32, char)>);

#[test]
fn test_unnamed_fields_top_field_attrs() {
    let s = Struct4::default();

    assert_eq!(s.0, 1);
    assert_eq!(s.1, "aaa");
    assert_eq!(s.2, "bbb");
    assert_eq!(s.3, [1, 2, 3, 4]);
    assert_eq!(s.4, vec![]);
}

#[derive(Default, PartialEq, Debug)]
struct Unit;

#[test]
fn test_unit() {
    let default = Unit::default();
    assert_eq!(Unit, default);
}
