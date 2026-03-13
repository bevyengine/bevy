use crate::{
    args::Args, commands::clippy_permutations::ClippyPermutations, Prepare, PreparedCommand,
};
use argh::FromArgs;

/// Check for clippy warnings and errors on `bevy_animation`.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "bevy_animation")]
pub struct BevyAnimation {}

impl Prepare for BevyAnimation {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        ClippyPermutations {
            crate_name: "bevy_animation",
            features: &[],
            all_features_features: &[],
        }
        .build::<Self>(sh, args)
    }
}
