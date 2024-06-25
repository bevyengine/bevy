use crate::{json::message_format_option, Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that the benches compile.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "bench-check")]
pub struct BenchCheckCommand {
    #[argh(switch)]
    /// emit errors as json
    emit_json: bool,
}

impl Prepare for BenchCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, _flags: Flag) -> Vec<PreparedCommand<'a>> {
        let format_flag = message_format_option(self.emit_json);

        vec![PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check --benches {format_flag} --target-dir ../target"
            ),
            "Failed to check the benches.",
        )
        .with_json(self.emit_json)]
    }
}
