use std::env::{self, set_current_dir};

use argh::FromArgs;

use super::{run_cargo_command, RustChannel};

/// Runs the compile-fail tests.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "compile-fail")]
pub struct CompileFailCommand {}

impl CompileFailCommand {}

impl CompileFailCommand {
    const FLAGS: &'static [&'static str] = &["--target-dir", "../../../target"];
    const ENV_VARS: &'static [(&'static str, &'static str)] = &[];

    /// Runs this command.
    ///
    /// For use in aliases.
    pub fn run_with_intermediate(no_fail_fast: bool) -> Result<(), ()> {
        let base_dir = env::current_dir().map_err(|err| eprintln!("{err}"))?;

        let mut flags = Self::FLAGS.to_vec();
        if no_fail_fast {
            flags.push("--no-fail-fast");
        }

        set_current_dir("crates/bevy_derive/compile_fail").map_err(|err| eprintln!("{err}"))?;
        let compile_fail_result =
            run_cargo_command("test", RustChannel::Stable, &flags, Self::ENV_VARS);
        set_current_dir(&base_dir).map_err(|err| eprintln!("{err}"))?;
        if !no_fail_fast && compile_fail_result.is_err() {
            return compile_fail_result;
        }

        set_current_dir("crates/bevy_ecs/compile_fail").map_err(|err| eprintln!("{err}"))?;
        let compile_fail_result =
            run_cargo_command("test", RustChannel::Stable, &flags, Self::ENV_VARS);
        set_current_dir(&base_dir).map_err(|err| eprintln!("{err}"))?;
        if !no_fail_fast && compile_fail_result.is_err() {
            return compile_fail_result;
        }

        set_current_dir("crates/bevy_reflect/compile_fail").map_err(|err| eprintln!("{err}"))?;
        let compile_fail_result =
            run_cargo_command("test", RustChannel::Stable, &flags, Self::ENV_VARS);
        set_current_dir(&base_dir).map_err(|err| eprintln!("{err}"))?;
        compile_fail_result
    }

    /// Runs this command.
    pub fn run(self, no_fail_fast: bool) -> Result<(), ()> {
        Self::run_with_intermediate(no_fail_fast)
    }
}
