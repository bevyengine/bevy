#[test]
fn test() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/reflect_remote/*.fail.rs");
    t.pass("tests/reflect_remote/*.pass.rs");
}
