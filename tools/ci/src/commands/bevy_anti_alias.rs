use crate::{
    args::Args, commands::clippy_permutations::ClippyPermutations, Prepare, PreparedCommand,
};
use argh::FromArgs;

/// Check for clippy warnings and errors on `bevy_anti_alias`.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "bevy_anti_alias")]
pub struct BevyAntiAlias {}

impl Prepare for BevyAntiAlias {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        ClippyPermutations {
            crate_name: "bevy_anti_alias",
            features: &[
                "trace",
                "smaa_luts bevy_image/zstd_rust",
                "smaa_luts bevy_image/zstd_c",
                "dlss force_disable_dlss",
            ],
            all_features_features: &["bevy_image/zstd_rust", "bevy_image/zstd_c"],
        }
        .build::<Self>(sh, args)
    }
}
