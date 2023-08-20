#[test]
fn test() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/deref_mut_derive/*.fail.rs");
    t.pass("tests/deref_mut_derive/*.pass.rs");
}
