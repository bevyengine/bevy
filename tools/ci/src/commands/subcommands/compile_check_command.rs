use crate::commands::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that the project compiles.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "compile-check")]
pub(crate) struct CompileCheckCommand {
    /// runs the check with the `--no-fail-fast` flag
    #[argh(switch, hidden_help)]
    keep_going: bool,
}

impl Prepare for CompileCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        let mut args = vec!["--workspace"];
        if flags.contains(Flag::KEEP_GOING) || self.keep_going {
            args.push("--no-fail-fast");
        }

        vec![PreparedCommand::new::<Self>(
            cmd!(sh, "cargo check {args...}"),
            "Please fix compiler errors in output above.",
        )]
    }
}
