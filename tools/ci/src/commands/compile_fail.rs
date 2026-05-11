use crate::{args::Args, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Runs the compile-fail tests.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "compile-fail")]
pub struct CompileFailCommand {}

impl Prepare for CompileFailCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let no_fail_fast = args.keep_going();
        let jobs = args.build_jobs();
        let test_threads = args.test_threads();
        let jobs_ref = jobs.as_ref();
        let test_threads_ref = test_threads.as_ref();

        let mut commands = vec![];

        // Macro Compile Fail Tests
        // Run tests (they do not get executed with the workspace tests)
        // - See crates/bevy_macros/compile_fail/README.md
        commands.push(
            PreparedCommand::new::<Self>(
                cmd!(sh, "cargo test {no_fail_fast...} {jobs_ref...} --manifest-path crates/bevy_derive/compile_fail/Cargo.toml -- {test_threads_ref...}"),
                "Compiler errors of the macros compile fail tests seem to be different than expected! Check locally and compare rust versions.",
            ),
        );

        // ECS Compile Fail Tests
        // Run UI tests (they do not get executed with the workspace tests)
        // - See crates/bevy_ecs/compile_fail/README.md
        commands.push(
            PreparedCommand::new::<Self>(
                cmd!(sh, "cargo test {no_fail_fast...} {jobs_ref...} --manifest-path crates/bevy_ecs/compile_fail/Cargo.toml -- {test_threads_ref...}"),
                "Compiler errors of the ECS compile fail tests seem to be different than expected! Check locally and compare rust versions.",
            ),
        );

        // Reflect Compile Fail Tests
        // Run tests (they do not get executed with the workspace tests)
        // - See crates/bevy_reflect/compile_fail/README.md
        commands.push(
            PreparedCommand::new::<Self>(
                cmd!(sh, "cargo test {no_fail_fast...} {jobs...} --manifest-path crates/bevy_reflect/compile_fail/Cargo.toml -- {test_threads...}"),
                "Compiler errors of the Reflect compile fail tests seem to be different than expected! Check locally and compare rust versions.",
            ),
        );

        commands
    }
}
