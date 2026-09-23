//! Compile-fail tests for the ECS APIs.

fn main() -> compile_fail_utils::ui_test::Result<()> {
    compile_fail_utils::test("ecs_ui", "tests/ui")
}
