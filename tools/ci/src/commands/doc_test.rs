use crate::{args::Args, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Runs all doc tests.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "doc-test")]
pub struct DocTestCommand {}

impl Prepare for DocTestCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let no_fail_fast = args.keep_going();
        let jobs = args.build_jobs();
        let test_threads = args.test_threads();

        vec![PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo test --workspace --doc {no_fail_fast...} {jobs...} -- {test_threads...}"
            ),
            "Please fix failing doc tests in output above.",
        )]
    }
}
