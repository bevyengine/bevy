use crate::{json::message_format_option, Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that the examples compile.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "example-check")]
pub struct ExampleCheckCommand {
    #[argh(switch)]
    /// emit errors as json
    emit_json: bool,
}

impl Prepare for ExampleCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, _flags: Flag) -> Vec<PreparedCommand<'a>> {
        let format_flag = message_format_option(self.emit_json);

        vec![PreparedCommand::new::<Self>(
            cmd!(sh, "cargo check --workspace {format_flag} --examples"),
            "Please fix compiler errors for examples in output above.",
        )
        .with_json(self.emit_json)]
    }
}
