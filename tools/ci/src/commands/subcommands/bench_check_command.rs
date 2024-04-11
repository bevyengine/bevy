use crate::commands::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that the benches compile.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "bench-check")]
pub(crate) struct BenchCheckCommand {
    /// runs the check with the `--no-fail-fast` flag
    #[argh(switch, hidden_help)]
    keep_going: bool,
}

impl Prepare for BenchCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        let mut args = vec!["--benches", "--target-dir", "../target"];
        if flags.contains(Flag::KEEP_GOING) || self.keep_going {
            args.push("--no-fail-fast");
        }

        vec![PreparedCommand::new::<Self>(
            cmd!(sh, "cargo check {args...}"),
            "Failed to check the benches.",
        )]
    }
}
