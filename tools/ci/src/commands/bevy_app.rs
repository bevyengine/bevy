use crate::{
    args::Args, commands::clippy_permutations::ClippyPermutations, Prepare, PreparedCommand,
};
use argh::FromArgs;

/// Check for clippy warnings and errors on `bevy_app`.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "bevy_app")]
pub struct BevyApp {}

impl Prepare for BevyApp {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        ClippyPermutations {
            crate_name: "bevy_app",
            features: &[
                "bevy_reflect",
                "reflect_functions",
                "reflect_auto_register bevy_reflect/auto_register_inventory",
                "reflect_auto_register bevy_reflect/auto_register_static",
                "trace",
                "bevy_debug_stepping",
                "error_panic_hook",
                "std",
                "critical-section",
                "web",
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
