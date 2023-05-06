#[test]
fn test() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/reflect_derive/*.fail.rs");
    t.pass("tests/reflect_derive/*.pass.rs");
}
