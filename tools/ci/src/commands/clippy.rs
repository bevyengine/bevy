use crate::{args::Args, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Check for clippy warnings and errors.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "clippy")]
pub struct ClippyCommand {}

impl Prepare for ClippyCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let jobs = args.build_jobs();

        vec![PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo clippy --workspace --all-targets --all-features {jobs...} -- -Dwarnings"
            ),
            "Please fix clippy errors in output above.",
        )]
    }
}
