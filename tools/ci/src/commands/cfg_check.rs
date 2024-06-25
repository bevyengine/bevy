use crate::{json::message_format_option, Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that the project compiles using the nightly compiler with cfg checks enabled.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "cfg-check")]
pub struct CfgCheckCommand {
    #[argh(switch)]
    /// emit errors as json
    emit_json: bool,
}

impl Prepare for CfgCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, _flags: Flag) -> Vec<PreparedCommand<'a>> {
        let format_flag = message_format_option(self.emit_json);

        vec![PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo +nightly check {format_flag} -Zcheck-cfg --workspace"
            ),
            "Please fix failing cfg checks in output above.",
        )
        .with_env_var("RUSTFLAGS", "-D warnings")
        .with_json(self.emit_json)]
    }
}
