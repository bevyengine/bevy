use super::{run_cargo_command, run_cargo_command_with_json, RustToolchain};
use crate::json::JsonCommandOutput;
use argh::FromArgs;

/// Checks that the benches compile.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "bench-check")]
pub struct BenchCheckCommand {
    #[argh(switch)]
    /// emit errors as json
    emit_json: bool,
}

impl BenchCheckCommand {
    const FLAGS: &'static [&'static str] = &[
        "--benches",
        "--target-dir",
        "../target",
        "--manifest-path",
        "./benches/Cargo.toml",
    ];
    const ENV_VARS: &'static [(&'static str, &'static str)] = &[];

    /// Runs this command.
    ///
    /// For use in aliases.
    pub fn run_with_intermediate() -> Result<(), ()> {
        run_cargo_command("check", RustToolchain::Active, Self::FLAGS, Self::ENV_VARS)
    }

    /// Runs this command with json output.
    ///
    /// For use in aliases.
    pub fn run_with_intermediate_json() -> Result<JsonCommandOutput, ()> {
        run_cargo_command_with_json(
            "check",
            "bench-check",
            RustToolchain::Active,
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
