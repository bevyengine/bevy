use crate::{json::message_format_option, Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Runs all doc tests.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "doc-test")]
pub struct DocTestCommand {
    #[argh(switch)]
    /// emit errors as json
    emit_json: bool,
}

impl DocTestCommand {
    pub fn with_json(mut self, emit_json: bool) -> Self {
        self.emit_json = emit_json;
        self
    }
}

impl Prepare for DocTestCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        let no_fail_fast = flags
            .contains(Flag::KEEP_GOING)
            .then_some("--no-fail-fast")
            .unwrap_or_default();

        let format_flag = message_format_option(self.emit_json);

        vec![PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo test {format_flag} --workspace --doc {no_fail_fast}"
            ),
            "Please fix failing doc tests in output above.",
        )
        .with_json(self.emit_json)]
    }
}
