use crate::commands::{ClippyCommand, FormatCommand};
use argh::FromArgs;

/// Alias for running the `format` and `clippy` subcommands.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "lints")]
pub struct LintsCommand {}

impl LintsCommand {
    /// Runs this command.
    pub fn run(self, no_fail_fast: bool) -> Result<(), ()> {
        let format_result = FormatCommand::run_with_intermediate();

        if !no_fail_fast && format_result.is_err() {
            return format_result;
        }

        ClippyCommand::run_with_intermediate()
    }
}
