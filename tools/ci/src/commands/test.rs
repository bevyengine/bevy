use crate::{args::Args, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Runs all tests (except for doc tests).
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "test")]
pub struct TestCommand {}

impl Prepare for TestCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let no_fail_fast = args.keep_going();
        let jobs = args.build_jobs();
        let test_threads = args.test_threads();

        let jobs_ref = &jobs;
        let test_threads_ref = &test_threads;

        vec![
            PreparedCommand::new::<Self>(
                cmd!(
                    sh,
                    "cargo test --workspace --lib --bins --tests {no_fail_fast...} {jobs_ref...} -- {test_threads_ref...}"
                ),
                "Please fix failing tests in output above.",
            ),
            PreparedCommand::new::<Self>(
                cmd!(
                    sh,
                    // `--benches` runs each benchmark once in order to verify that they behave
                    // correctly and do not panic.
                    "cargo test --workspace --benches {no_fail_fast...} {jobs...}"
                ),
                "Please fix failing tests in output above.",
            )
        ]
    }
}
