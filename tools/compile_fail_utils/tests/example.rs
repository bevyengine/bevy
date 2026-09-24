//! Example compile-fail test, used to exercise `compile_fail_utils` itself.

// Run all tests in the tests/example_tests folder.
// If we had more tests we could either call this function
// on every single one or use test_multiple and past it an array
// of paths.
//
// Don't forget that when running tests the working directory
// is set to the crate root.
fn main() -> ui_test::Result<()> {
    compile_fail_utils::test("example_tests", "tests/example_tests")
}
