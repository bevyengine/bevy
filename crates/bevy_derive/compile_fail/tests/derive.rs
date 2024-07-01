fn main() -> compile_fail_utils::ui_test::Result<()> {
    compile_fail_utils::test_multiple(["tests/deref_derive", "tests/deref_mut_derive"])
}

