use crate::json::JsonCommandOutput;
use argh::FromArgs;

use super::{run_cargo_command, run_cargo_command_with_json, RustChannel};

/// Checks that the project compiles.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "compile-check")]
pub struct CompileCheckCommand {
    #[argh(switch)]
    /// emit errors as json
    emit_json: bool,
}

impl CompileCheckCommand {
    const FLAGS: &'static [&'static str] = &["--workspace"];
    const ENV_VARS: &'static [(&'static str, &'static str)] = &[];

    /// Runs this command.
    ///
    /// For use in aliases.
    pub fn run_with_intermediate() -> Result<(), ()> {
        run_cargo_command("check", RustChannel::Stable, Self::FLAGS, Self::ENV_VARS)
    }

    /// Runs this command with json output.
    ///
    /// For use in aliases.
    pub fn run_with_intermediate_json() -> Result<JsonCommandOutput, ()> {
        run_cargo_command_with_json(
            "check",
            "compile-check",
            RustChannel::Stable,
            Self::FLAGS,
            Self::ENV_VARS,
        )
    }

    /// Runs this command.
    pub fn run(self) -> Result<(), ()> {
        if self.emit_json {
            Self::run_with_intermediate_json().map(|json| {
                println!("[{}]", json.as_json_string());
            })
        } else {
            Self::run_with_intermediate()
        }
    }
}
