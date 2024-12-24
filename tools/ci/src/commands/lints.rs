use argh::FromArgs;

use crate::commands::{ClippyCommand, FormatCommand};

/// Alias for running the `format` and `clippy` subcommands.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "lints")]
pub struct LintsCommand {}

impl LintsCommand {
    /// Runs this command.
    ///
    /// For use in aliases.
    pub fn run_with_intermediate(no_fail_fast: bool) -> Result<(), ()> {
        let format_result = FormatCommand {}.run();

        if !no_fail_fast && format_result.is_err() {
            return format_result;
        }

        ClippyCommand::default().run()
    }

    /// Runs this command.
    pub fn run(self, no_fail_fast: bool) -> Result<(), ()> {
        Self::run_with_intermediate(no_fail_fast)
    }
}
