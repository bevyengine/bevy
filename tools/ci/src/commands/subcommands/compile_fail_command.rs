use crate::commands::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Runs the compile-fail tests.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "compile-fail")]
pub(crate) struct CompileFailCommand {
    /// runs the check with the `--no-fail-fast` flag
    #[argh(switch, hidden_help)]
    keep_going: bool,
}

impl Prepare for CompileFailCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        let mut args = vec!["--target-dir", "../../target"];
        if flags.contains(Flag::KEEP_GOING) || self.keep_going {
            args.push("--no-fail-fast");
        }

        let mut commands = vec![];

        // ECS Compile Fail Tests
        // Run UI tests (they do not get executed with the workspace tests)
        // - See crates/bevy_ecs_compile_fail_tests/README.md
        let args_clone = args.clone();
        commands.push(
            PreparedCommand::new::<Self>(
                cmd!(sh, "cargo test {args_clone...}"),
                "Compiler errors of the ECS compile fail tests seem to be different than expected! Check locally and compare rust versions.",
            )
            .with_subdir("crates/bevy_ecs_compile_fail_tests"),
        );

        // Reflect Compile Fail Tests
        // Run tests (they do not get executed with the workspace tests)
        // - See crates/bevy_reflect_compile_fail_tests/README.md
        let args_clone = args.clone();
        commands.push(
            PreparedCommand::new::<Self>(
                cmd!(sh, "cargo test {args_clone...}"),
                "Compiler errors of the Reflect compile fail tests seem to be different than expected! Check locally and compare rust versions.",
            )
            .with_subdir("crates/bevy_reflect_compile_fail_tests"),
        );

        // Macro Compile Fail Tests
        // Run tests (they do not get executed with the workspace tests)
        // - See crates/bevy_macros_compile_fail_tests/README.md
        let args_clone = args.clone();
        commands.push(
            PreparedCommand::new::<Self>(
                cmd!(sh, "cargo test {args_clone...}"),
                "Compiler errors of the macros compile fail tests seem to be different than expected! Check locally and compare rust versions.",
            )
            .with_subdir("crates/bevy_macros_compile_fail_tests"),
        );

        commands
    }
}
