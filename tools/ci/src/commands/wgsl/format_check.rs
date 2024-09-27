use super::{get_files::get_wgsl_files, install::WgslInstallCommand};
use crate::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Check code formatting.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "wgsl-format-check")]
pub struct WgslFormatCheckCommand {}

impl Prepare for WgslFormatCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        // Collect all .wgsl files from the workspace
        let files = get_wgsl_files();
        let mut commands = vec![];
        commands.append(&mut WgslInstallCommand::default().prepare(sh, flags));
        commands.append(&mut vec![PreparedCommand::new::<Self>(
            cmd!(sh, "wgslfmt {files...} --check"),
            "Please run 'cargo run -p ci -- wgsl-format' to format your code.",
        )]);
        commands
    }
}
