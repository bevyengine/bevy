use crate::{json::message_format_option, Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that all tests compile.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "test-check")]
pub struct TestCheckCommand {
    #[argh(switch)]
    /// emit errors as json
    emit_json: bool,
}

impl Prepare for TestCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, _flags: Flag) -> Vec<PreparedCommand<'a>> {
        let format_flag = message_format_option(self.emit_json);

        vec![PreparedCommand::new::<Self>(
            cmd!(sh, "cargo check {format_flag} --workspace --tests"),
            "Please fix compiler examples for tests in output above.",
        )
        .with_json(self.emit_json)]
    }
}
