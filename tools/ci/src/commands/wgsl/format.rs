use super::install::WgslInstallCommand;
use crate::{args::Args, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Check code formatting.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "wgsl-format")]
pub struct WgslFormatCommand {}

impl Prepare for WgslFormatCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        // Collect all .wgsl files from the workspace
        let mut commands = vec![];
        // wgslfmt can take multiple runs to fully format a file
        let format_command =
            PreparedCommand::new::<Self>(cmd!(sh, "wgslfmt"), "wgslfmt failed unexpectedly.");
        commands.append(&mut WgslInstallCommand::default().prepare(sh, args));
        commands.push(format_command);
        commands
    }
}
