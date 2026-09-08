use crate::{args::Args, commands::*, Prepare, PreparedCommand};
use argh::FromArgs;

/// Check for clippy warnings and errors running multiple permutations of features.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "clippys")]
pub struct ClippysCommand {}

impl Prepare for ClippysCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let mut commands = Vec::new();

        commands.append(&mut BevyA11y::default().prepare(sh, args));
        commands.append(&mut BevyAnimation::default().prepare(sh, args));
        commands.append(&mut BevyAntiAlias::default().prepare(sh, args));
        commands.append(&mut BevyApp::default().prepare(sh, args));
        commands.append(&mut BevyEcs::default().prepare(sh, args));

        commands
    }
}
