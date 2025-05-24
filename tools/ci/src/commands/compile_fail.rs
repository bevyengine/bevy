use crate::{args::Args, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Runs the compile-fail tests.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "compile-fail")]
pub struct CompileFailCommand {}

impl Prepare for CompileFailCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let no_fail_fast = args.keep_going.then_some("--no-fail-fast");
        let jobs = args.build_jobs.map(|jobs| format!("--jobs={jobs}"));
        let test_threads = args
            .test_threads
            .map(|threads| format!("--test-threads={threads}"));

        let mut commands = vec![];

        // Macro Compile Fail Tests
        // Run tests (they do not get executed with the workspace tests)
        // - See crates/bevy_macros_compile_fail_tests/README.md
        let jobs_clone = jobs.clone();
        let test_threads_clone = test_threads.clone();
        commands.push(
            PreparedCommand::new::<Self>(
                cmd!(sh, "cargo test --target-dir ../../../target {no_fail_fast...} {jobs_clone...} -- {test_threads_clone...}"),
                "Compiler errors of the macros compile fail tests seem to be different than expected! Check locally and compare rust versions.",
            )
            .with_subdir("crates/bevy_derive/compile_fail"),
        );

        // ECS Compile Fail Tests
        // Run UI tests (they do not get executed with the workspace tests)
        // - See crates/bevy_ecs_compile_fail_tests/README.md
        let jobs_clone = jobs.clone();
        let test_threads_clone = test_threads.clone();
        commands.push(
            PreparedCommand::new::<Self>(
                cmd!(sh, "cargo test --target-dir ../../../target {no_fail_fast...} {jobs_clone...} -- {test_threads_clone...}"),
                "Compiler errors of the ECS compile fail tests seem to be different than expected! Check locally and compare rust versions.",
            )
            .with_subdir("crates/bevy_ecs/compile_fail"),
        );

        // Reflect Compile Fail Tests
        // Run tests (they do not get executed with the workspace tests)
        // - See crates/bevy_reflect_compile_fail_tests/README.md
        commands.push(
            PreparedCommand::new::<Self>(
                cmd!(sh, "cargo test --target-dir ../../../target {no_fail_fast...} {jobs...} -- {test_threads...}"),
                "Compiler errors of the Reflect compile fail tests seem to be different than expected! Check locally and compare rust versions.",
            )
            .with_subdir("crates/bevy_reflect/compile_fail"),
        );

        commands
    }
}
