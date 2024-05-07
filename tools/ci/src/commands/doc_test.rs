use crate::json::JsonCommandOutput;
use argh::FromArgs;

use super::{run_cargo_command, run_cargo_command_with_json, RustChannel};

/// Runs all doc tests.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "doc-test")]
pub struct DocTestCommand {
    #[argh(switch)]
    /// emit errors as json
    emit_json: bool,
}

impl DocTestCommand {
    const FLAGS: &'static [&'static str] = &["--workspace", "--doc"];
    const ENV_VARS: &'static [(&'static str, &'static str)] = &[];

    /// Runs this command.
    ///
    /// For use in aliases.
    pub fn run_with_intermediate(no_fail_fast: bool) -> Result<(), ()> {
        let mut flags = Self::FLAGS.to_vec();
        if no_fail_fast {
            flags.push("--no-fail-fast");
        }

        run_cargo_command("doc", RustChannel::Stable, &flags, Self::ENV_VARS)
    }

    /// Runs this command with json output.
    ///
    /// For use in aliases.
    pub fn run_with_intermediate_json(no_fail_fast: bool) -> Result<JsonCommandOutput, ()> {
        let mut flags = Self::FLAGS.to_vec();
        if no_fail_fast {
            flags.push("--no-fail-fast");
        }

        run_cargo_command_with_json(
            "doc",
            "doc-test",
            RustChannel::Stable,
            &flags,
            Self::ENV_VARS,
        )
    }

    /// Runs this command.
    pub fn run(self, no_fail_fast: bool) -> Result<(), ()> {
        if self.emit_json {
            Self::run_with_intermediate_json(no_fail_fast).map(|json| {
                println!("[{}]", json.as_json_string());
            })
        } else {
            Self::run_with_intermediate(no_fail_fast)
        }
    }
}
