use crate::{args::Args, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that all docs compile.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "doc-check")]
pub struct DocCheckCommand {}

impl Prepare for DocCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let jobs = args.build_jobs();

        vec![PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo doc --workspace --all-features --no-deps --document-private-items {jobs...} --keep-going"
            ),
            "Please fix doc warnings in output above.",
        )
        .with_env_var("RUSTDOCFLAGS", "-D warnings")]
    }
}
