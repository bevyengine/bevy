use crate::{args::Args, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that the benches compile.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "bench-check")]
pub struct BenchCheckCommand {}

impl Prepare for BenchCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let jobs = args
            .build_jobs
            .map(|jobs| format!(" --jobs {jobs}"))
            .unwrap_or_default();

        vec![PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check --benches --target-dir ../target --manifest-path ./benches/Cargo.toml{jobs}"
            ),
            "Failed to check the benches.",
        )]
    }
}
