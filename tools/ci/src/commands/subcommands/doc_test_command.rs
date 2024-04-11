use crate::commands::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Runs all doc tests.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "doc-test")]
pub(crate) struct DocTestCommand {
    /// runs the check with the `--no-fail-fast` flag
    #[argh(switch, hidden_help)]
    keep_going: bool,
}

impl Prepare for DocTestCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        let mut args = vec!["--workspace", "--doc"];
        if flags.contains(Flag::KEEP_GOING) || self.keep_going {
            args.push("--no-fail-fast");
        }

        vec![PreparedCommand::new::<Self>(
            cmd!(sh, "cargo test {args...}"),
            "Please fix failing doc tests in output above.",
        )]
    }
}
