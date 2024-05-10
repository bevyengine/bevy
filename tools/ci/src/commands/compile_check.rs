use crate::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that the project compiles.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "compile-check")]
pub struct CompileCheckCommand {}

impl Prepare for CompileCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        let quiet = flags
            .contains(Flag::QUIET)
            .then_some("--quiet")
            .unwrap_or_default();

        vec![PreparedCommand::new::<Self>(
            cmd!(sh, "cargo check --workspace {quiet}"),
            "Please fix compiler errors in output above.",
        )]
    }
}
