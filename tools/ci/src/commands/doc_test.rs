use crate::{args::Args, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Runs all doc tests.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "doc-test")]
pub struct DocTestCommand {}

impl Prepare for DocTestCommand {
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
            .map(|test_threads| format!("--test-threads={test_threads}"))
            .unwrap_or_default();

        vec![PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo test --workspace --doc {no_fail_fast}{jobs} -- {test_threads}"
            ),
            "Please fix failing doc tests in output above.",
        )]
    }
}
