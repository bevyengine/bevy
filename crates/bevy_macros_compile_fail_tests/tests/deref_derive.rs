#[test]
fn test() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/deref_derive/*.fail.rs");
    t.pass("tests/deref_derive/*.pass.rs");
}
