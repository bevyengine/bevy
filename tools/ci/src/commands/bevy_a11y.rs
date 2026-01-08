use crate::{
    args::Args, commands::clippy_permutations::ClippyPermutations, Prepare, PreparedCommand,
};
use argh::FromArgs;

/// Check for clippy warnings and errors on `bevy_a11y`.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "bevy_a11y")]
pub struct BevyA11y {}

impl Prepare for BevyA11y {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        ClippyPermutations {
            crate_name: "bevy_a11y",
            features: &["bevy_reflect", "serialize", "std", "critical-section"],
            all_features_features: &[],
        }
        .build::<Self>(sh, args)
    }
}
