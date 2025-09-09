use crate::{args::Args, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that the benches compile.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "bench-check")]
pub struct BenchCheckCommand {}

impl Prepare for BenchCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let jobs = args.build_jobs();

        vec![PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check --benches {jobs...} --target-dir ../target --manifest-path ./benches/Cargo.toml"
            ),
            "Failed to check the benches.",
        )]
    }
}
