use crate::commands::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that all tests compile.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "test-check")]
pub(crate) struct TestCheckCommand {
    /// runs the check with the `--no-fail-fast` flag
    #[argh(switch, hidden_help)]
    keep_going: bool,
}

impl Prepare for TestCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        let mut args = vec!["--workspace", "--tests"];
        if flags.contains(Flag::KEEP_GOING) || self.keep_going {
            args.push("--no-fail-fast");
        }

        vec![PreparedCommand::new::<Self>(
            cmd!(sh, "cargo check {args...}"),
            "Please fix compiler examples for tests in output above.",
        )]
    }
}
