use crate::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Runs the compile-fail tests.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "compile-fail")]
pub struct CompileFailCommand {}

impl Prepare for CompileFailCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        let no_fail_fast = flags
            .contains(Flag::KEEP_GOING)
            .then_some("--no-fail-fast")
            .unwrap_or_default();

        let mut commands = vec![];

        // Macro Compile Fail Tests
        // Run tests (they do not get executed with the workspace tests)
        // - See crates/bevy_macros_compile_fail_tests/README.md
        commands.push(
            PreparedCommand::new::<Self>(
                cmd!(sh, "cargo test --target-dir ../../../target {no_fail_fast}"),
                "Compiler errors of the macros compile fail tests seem to be different than expected! Check locally and compare rust versions.",
            )
            .with_subdir("crates/bevy_derive/compile_fail"),
        );

        // ECS Compile Fail Tests
        // Run UI tests (they do not get executed with the workspace tests)
        // - See crates/bevy_ecs_compile_fail_tests/README.md
        commands.push(
            PreparedCommand::new::<Self>(
                cmd!(sh, "cargo test --target-dir ../../../target {no_fail_fast}"),
                "Compiler errors of the ECS compile fail tests seem to be different than expected! Check locally and compare rust versions.",
            )
            .with_subdir("crates/bevy_ecs/compile_fail"),
        );

        // Reflect Compile Fail Tests
        // Run tests (they do not get executed with the workspace tests)
        // - See crates/bevy_reflect_compile_fail_tests/README.md
        commands.push(
            PreparedCommand::new::<Self>(
                cmd!(sh, "cargo test --target-dir ../../../target {no_fail_fast}"),
                "Compiler errors of the Reflect compile fail tests seem to be different than expected! Check locally and compare rust versions.",
            )
            .with_subdir("crates/bevy_reflect/compile_fail"),
        );

        commands
    }
}
