#[test]
fn test() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/field_attributes/*.fail.rs");
    t.pass("tests/field_attributes/*.pass.rs");
}
