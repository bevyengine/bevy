use crate::{args::Args, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Check for clippy warnings and errors for crates/features that require Dlss SDK.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "clippy_dlss")]
pub struct ClippyDlssCommand {}

impl Prepare for ClippyDlssCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let jobs = args.build_jobs();

        vec![PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo clippy -p bevy_anti_alias --no-default-features --features=dlss {jobs...} -- -Dwarnings"
            ),
            "Please fix clippy errors in output above.",
        )]
    }
}
