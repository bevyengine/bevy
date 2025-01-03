use crate::{args::Args, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Check code formatting.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "format")]
pub struct FormatCommand {}

impl Prepare for FormatCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let jobs = args
            .jobs
            .map(|jobs| format!(" --jobs{jobs}"))
            .unwrap_or_default();

        vec![PreparedCommand::new::<Self>(
            cmd!(sh, "cargo fmt --all{jobs} -- --check"),
            "Please run 'cargo fmt --all' to format your code.",
        )]
    }
}
