use crate::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that the project compiles using the nightly compiler with cfg checks enabled.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "cfg-check")]
pub struct CfgCheckCommand {}

impl Prepare for CfgCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, _flags: Flag) -> Vec<PreparedCommand<'a>> {
        vec![PreparedCommand::new::<Self>(
            cmd!(sh, "cargo +nightly check -Zcheck-cfg --workspace"),
            "Please fix failing cfg checks in output above.",
        )
        .with_env_var("RUSTFLAGS", "-D warnings")]
    }
}
