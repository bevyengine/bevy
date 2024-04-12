use crate::commands::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that the examples compile.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "example-check")]
pub(crate) struct ExampleCheckCommand {}

impl Prepare for ExampleCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        let mut args = vec!["--workspace", "--examples"];
        if flags.contains(Flag::KEEP_GOING) {
            args.push("--no-fail-fast");
        }

        vec![PreparedCommand::new::<Self>(
            cmd!(sh, "cargo check {args...}"),
            "Please fix compiler errors for examples in output above.",
        )]
    }
}
