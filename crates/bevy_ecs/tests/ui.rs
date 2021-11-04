#[rustversion::attr(nightly, ignore)]
#[test]
fn test() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
