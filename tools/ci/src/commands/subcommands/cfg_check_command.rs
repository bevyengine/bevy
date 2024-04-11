use crate::commands::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that the project compiles using the nightly compiler with cfg checks enabled.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "cfg-check")]
pub(crate) struct CfgCheckCommand {
    /// runs the check with the `--no-fail-fast` flag
    #[argh(switch, hidden_help)]
    keep_going: bool,
}

impl Prepare for CfgCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        let mut args = vec!["-Zcheck-cfg", "--workspace"];
        if flags.contains(Flag::KEEP_GOING) || self.keep_going {
            args.push("--no-fail-fast");
        }

        vec![PreparedCommand::new::<Self>(
            cmd!(sh, "cargo +nightly check {args...}"),
            "Please fix failing cfg checks in output above.",
        )
        .with_env_var("RUSTFLAGS", "-D warnings")]
    }
}
