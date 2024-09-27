use super::{get_files::get_wgsl_files, install::WgslInstallCommand};
use crate::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Check code formatting.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "wgsl-format")]
pub struct WgslFormatCommand {}

/// Number of times to repeat the format operation.
const REPETITIONS: usize = 5;

impl Prepare for WgslFormatCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        // Collect all .wgsl files from the workspace
        let files = &get_wgsl_files();
        let mut commands = vec![];
        // wgslfmt can take multiple runs to fully format a file
        let mut format_command: Vec<_> = core::iter::from_fn(|| {
            Some(PreparedCommand::new::<Self>(
                cmd!(sh, "wgslfmt {files...}"),
                "wgslfmt failed unexpectedly.",
            ))
        })
        .take(REPETITIONS)
        .collect();
        commands.append(&mut WgslInstallCommand::default().prepare(sh, flags));
        commands.append(&mut format_command);
        commands
    }
}
