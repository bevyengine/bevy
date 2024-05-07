use argh::FromArgs;

use super::{run_cargo_command, RustChannel};

/// Check code formatting.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "format")]
pub struct FormatCommand {}

impl FormatCommand {
    const FLAGS: &'static [&'static str] = &["--all", "--", "--check"];
    const ENV_VARS: &'static [(&'static str, &'static str)] = &[];

    /// Runs this command.
    ///
    /// For use in aliases.
    pub fn run_with_intermediate() -> Result<(), ()> {
        run_cargo_command("fmt", RustChannel::Stable, Self::FLAGS, Self::ENV_VARS)
    }

    /// Runs this command.
    pub fn run(self) -> Result<(), ()> {
        Self::run_with_intermediate()
    }
}
