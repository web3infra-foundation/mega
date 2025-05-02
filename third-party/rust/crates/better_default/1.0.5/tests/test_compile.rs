#[test]
fn test_compile() {
    let test_case = trybuild::TestCases::new();

    test_case.pass("./tests/compile_pass/*/*.rs");
    test_case.pass("./tests/compile_pass/*.rs");

    test_case.compile_fail("./tests/compile_fail/*/*.rs");
    test_case.compile_fail("./tests/compile_fail/*.rs");
}
