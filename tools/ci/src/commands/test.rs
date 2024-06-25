use crate::{json::message_format_option, Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Runs all tests (except for doc tests).
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "test")]
pub struct TestCommand {
    #[argh(switch)]
    /// emit errors as json
    emit_json: bool,
}

impl Prepare for TestCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        let no_fail_fast = flags
            .contains(Flag::KEEP_GOING)
            .then_some("--no-fail-fast")
            .unwrap_or_default();

        let format_flag = message_format_option(self.emit_json);

        vec![PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo test {format_flag} --workspace --lib --bins --tests --benches {no_fail_fast}"
            ),
            "Please fix failing tests in output above.",
        )
        .with_json(self.emit_json)]
    }
}
