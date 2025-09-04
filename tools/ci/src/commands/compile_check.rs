use crate::{args::Args, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that the project compiles.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "compile-check")]
pub struct CompileCheckCommand {}

impl Prepare for CompileCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let jobs = args.build_jobs();

        vec![PreparedCommand::new::<Self>(
            cmd!(sh, "cargo check --workspace {jobs...}"),
            "Please fix compiler errors in output above.",
        )]
    }
}
