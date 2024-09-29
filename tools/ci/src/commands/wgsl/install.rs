use crate::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Check code formatting.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "wgsl-format")]
pub struct WgslInstallCommand {}

impl Prepare for WgslInstallCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, _flags: Flag) -> Vec<PreparedCommand<'a>> {
        vec![PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo install --git https://github.com/wgsl-analyzer/wgsl-analyzer wgslfmt"
            ),
            "Failed to install wgslfmt.",
        )]
    }
}
