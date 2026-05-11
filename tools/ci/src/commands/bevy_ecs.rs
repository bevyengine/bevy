use crate::{
    args::Args, commands::clippy_permutations::ClippyPermutations, Prepare, PreparedCommand,
};
use argh::FromArgs;

/// Check for clippy warnings and errors on `bevy_ecs`.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "bevy_ecs")]
pub struct BevyEcs {}

impl Prepare for BevyEcs {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        ClippyPermutations {
            crate_name: "bevy_ecs",
            features: &[
                //"multi_threaded",
                "serialize",
                "bevy_reflect",
                "reflect_functions",
                "reflect_auto_register bevy_reflect/auto_register_inventory",
                "reflect_auto_register bevy_reflect/auto_register_static",
                "backtrace",
                "trace",
                "detailed_trace",
                "track_location",
                "async_executor",
                "std",
                "critical-section",
                "hotpatching",
            ],
            all_features_features: &[
                "bevy_reflect/auto_register_inventory",
                "bevy_reflect/auto_register_static",
            ],
        }
        .build::<Self>(sh, args)
    }
}
