use crate::{
    args::Args, commands::clippy_permutations::ClippyPermutations, Prepare, PreparedCommand,
};
use argh::FromArgs;

/// Check for clippy warnings and errors on `bevy_android`.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "bevy_android")]
pub struct BevyAndroid {}

impl Prepare for BevyAndroid {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        if !args.is_android_target() {
            panic!("BevyAndroid requires an Android target.");
        }

        ClippyPermutations {
            crate_name: "bevy_android",
            features: &[],
            all_features_features: &[],
        }
        .build::<Self>(sh, args)
    }
}
