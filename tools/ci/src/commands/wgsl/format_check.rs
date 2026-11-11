use super::install::WgslInstallCommand;
use crate::{args::Args, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Check code formatting.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "wgsl-format-check")]
pub struct WgslFormatCheckCommand {}

impl Prepare for WgslFormatCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        // Collect all .wgsl files from the workspace
        let mut commands = vec![];
        commands.append(&mut WgslInstallCommand::default().prepare(sh, args));
        commands.append(&mut vec![PreparedCommand::new::<Self>(
            cmd!(sh, "wgslfmt --check"),
            "Please run 'cargo run -p ci -- wgsl-format' to format your code.",
        )]);
        commands
    }
}
