#[test]
fn test() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/type_uuid_derive/*.rs");
}
