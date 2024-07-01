use crate::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that the benches compile.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "bench-check")]
pub struct BenchCheckCommand {}

impl Prepare for BenchCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, _flags: Flag) -> Vec<PreparedCommand<'a>> {
        vec![PreparedCommand::new::<Self>(
            cmd!(sh, "cargo check --benches --target-dir ../target"),
            "Failed to check the benches.",
        )]
    }
}
