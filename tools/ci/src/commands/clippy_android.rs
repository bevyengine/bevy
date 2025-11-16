use crate::{args::Args, commands::BevyAndroid, Prepare, PreparedCommand};
use argh::FromArgs;

/// Check for clippy warnings and errors for Android targets.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "clippy_android")]
pub struct ClippyAndroidCommand {}

impl Prepare for ClippyAndroidCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        if !args.is_android_target() {
            panic!("ClippyAndroidCommand requires an Android target.");
        }

        let mut commands = Vec::new();

        commands.append(&mut BevyAndroid::default().prepare(sh, args));

        commands
    }
}
