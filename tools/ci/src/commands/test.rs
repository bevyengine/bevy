use crate::{args::Args, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Runs all tests (except for doc tests).
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "test")]
pub struct TestCommand {}

impl Prepare for TestCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let no_fail_fast = args
            .keep_going
            .then_some("--no-fail-fast")
            .unwrap_or_default();

        let jobs = args
            .build_jobs
            .map(|jobs| format!(" --jobs {jobs}"))
            .unwrap_or_default();

        let test_threads = args
            .test_threads
            .map(|test_threads| format!(" -- --test-threads={test_threads}"))
            .unwrap_or_default();

        vec![PreparedCommand::new::<Self>(
            cmd!(
                sh,
                // `--benches` runs each benchmark once in order to verify that they behave
                // correctly and do not panic.
                "cargo test --workspace --lib --bins --tests --benches {no_fail_fast}{jobs}{test_threads}"
            ),
            "Please fix failing tests in output above.",
        )]
    }
}
