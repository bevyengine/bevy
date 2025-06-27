use crate::{args::Args, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Runs all tests (except for doc tests).
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "test-render-assets")]
pub struct TestRenderAssetsCommand {}

impl Prepare for TestRenderAssetsCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let no_fail_fast = args.keep_going();
        let jobs = args.build_jobs();
        let test_threads = args.test_threads();
        let jobs_ref = jobs.as_ref();
        let test_threads_ref = test_threads.as_ref();

        vec![PreparedCommand::new::<Self>(
            cmd!(sh, "cargo test --test render_asset_leaks {no_fail_fast...} {jobs_ref...} -- {test_threads_ref...}"),
            "Please fix failing tests in output above.",
        )]
    }
}
