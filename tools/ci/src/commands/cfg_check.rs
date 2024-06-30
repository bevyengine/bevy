use super::{run_cargo_command, run_cargo_command_with_json, RustToolchain};
use crate::json::JsonCommandOutput;
use argh::FromArgs;

/// Checks that the project compiles using the nightly compiler with cfg checks enabled.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "cfg-check")]
pub struct CfgCheckCommand {
    #[argh(switch)]
    /// emit errors as json
    emit_json: bool,
}

impl CfgCheckCommand {
    const FLAGS: &'static [&'static str] = &["-Zcheck-cfg", "--workspace"];
    const ENV_VARS: &'static [(&'static str, &'static str)] = &[("RUSTFLAGS", "-D warnings")];

    /// Runs this command.
    ///
    /// For use in aliases.
    pub fn run_with_intermediate() -> Result<(), ()> {
        run_cargo_command("check", RustToolchain::Nightly, Self::FLAGS, Self::ENV_VARS)
    }

    /// Runs this command with json output.
    ///
    /// For use in aliases.
    pub fn run_with_intermediate_json() -> Result<JsonCommandOutput, ()> {
        run_cargo_command_with_json(
            "check",
            "cfg-check",
            RustToolchain::Nightly,
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
