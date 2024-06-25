use crate::{json::message_format_option, Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that all docs compile.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "doc-check")]
pub struct DocCheckCommand {
    #[argh(switch)]
    /// emit errors as json
    emit_json: bool,
}

impl DocCheckCommand {
    pub fn with_json(mut self, emit_json: bool) -> Self {
        self.emit_json = emit_json;
        self
    }
}

impl Prepare for DocCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, _flags: Flag) -> Vec<PreparedCommand<'a>> {
        let format_flag = message_format_option(self.emit_json);

        vec![PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo doc --workspace {format_flag} --all-features --no-deps --document-private-items"
            ),
            "Please fix doc warnings in output above.",
        ).with_json(self.emit_json)]
    }
}
