fn main() -> bevy_compile_test_utils::ui_test::Result<()> {
    bevy_compile_test_utils::test_multiple(["tests/deref_derive", "tests/deref_mut_derive"])
}

