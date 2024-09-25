use crate::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that all docs compile on nightly.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "doc-check-nightly")]
pub struct DocCheckNightlyCommand {}

impl Prepare for DocCheckNightlyCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, _flags: Flag) -> Vec<PreparedCommand<'a>> {
        vec![PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo +nightly doc --workspace --all-features --no-deps --document-private-items --keep-going"
            ),
            "Please fix the warnings and errors in the above output.",
        )
        .with_env_var("RUSTDOCFLAGS", "-D warnings --cfg=docsrs")
        .with_env_var("RUSTFLAGS", "--cfg docsrs_dep")]
    }
}
