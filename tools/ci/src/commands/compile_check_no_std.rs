use crate::{args::Args, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that the project compiles for a `no_std` target.
/// Note that this tool will attempt to install the target via rustup.
/// This can be skipped by passing the "--skip-install" flag.
#[derive(FromArgs)]
#[argh(subcommand, name = "compile-check-no-std")]
pub struct CompileCheckNoStdCommand {
    /// the target to check against.
    /// Defaults to "x86_64-unknown-none"
    #[argh(option, default = "Self::default().target")]
    target: String,
    /// skip attempting the installation of the target.
    #[argh(switch)]
    skip_install: bool,
}

impl Default for CompileCheckNoStdCommand {
    fn default() -> Self {
        Self {
            target: String::from("x86_64-unknown-none"),
            skip_install: false,
        }
    }
}

impl Prepare for CompileCheckNoStdCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let jobs = args
            .build_jobs
            .map(|jobs| format!(" --jobs {jobs}"))
            .unwrap_or_default();

        let target = self.target.as_str();
        let mut commands = Vec::new();

        if !self.skip_install {
            commands.push(PreparedCommand::new::<Self>(
                cmd!(sh, "rustup target add {target}"),
                "Unable to add the required target via rustup, is it spelled correctly?",
            ));
        }

        commands.push(PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check -p bevy_ptr --no-default-features --target {target}{jobs}"
            ),
            "Please fix compiler errors in output above for bevy_ptr no_std compatibility.",
        ));

        commands.push(PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check -p bevy_utils --no-default-features --target {target}{jobs}"
            ),
            "Please fix compiler errors in output above for bevy_utils no_std compatibility.",
        ));

        commands.push(PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check -p bevy_mikktspace --no-default-features --features libm --target {target}{jobs}"
            ),
            "Please fix compiler errors in output above for bevy_mikktspace no_std compatibility.",
        ));

        commands.push(PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check -p bevy_reflect --no-default-features --target {target}"
            ),
            "Please fix compiler errors in output above for bevy_reflect no_std compatibility.",
        ));

        commands.push(PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check -p bevy_math --no-default-features --features libm --target {target}"
            ),
            "Please fix compiler errors in output above for bevy_math no_std compatibility.",
        ));

        commands.push(PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check -p bevy_color --no-default-features --features libm --target {target}"
            ),
            "Please fix compiler errors in output above for bevy_color no_std compatibility.",
        ));

        commands.push(PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check -p bevy_tasks --no-default-features --features edge_executor,critical-section --target {target}"
            ),
            "Please fix compiler errors in output above for bevy_tasks no_std compatibility.",
        ));

        commands.push(PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check -p bevy_ecs --no-default-features --features edge_executor,critical-section,bevy_debug_stepping,bevy_reflect --target {target}"
            ),
            "Please fix compiler errors in output above for bevy_ecs no_std compatibility.",
        ));

        commands.push(PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check -p bevy_app --no-default-features --features bevy_reflect --target {target}"
            ),
            "Please fix compiler errors in output above for bevy_app no_std compatibility.",
        ));

        commands.push(PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check -p bevy_hierarchy --no-default-features --features bevy_app,reflect --target {target}"
            ),
            "Please fix compiler errors in output above for bevy_hierarchy no_std compatibility.",
        ));

        commands.push(PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check -p bevy_input --no-default-features --features libm,serialize,bevy_reflect --target {target}"
            ),
            "Please fix compiler errors in output above for bevy_input no_std compatibility.",
        ));

        commands.push(PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check -p bevy_state --no-default-features --features bevy_reflect,bevy_app,bevy_hierarchy --target {target}"
            ),
            "Please fix compiler errors in output above for bevy_state no_std compatibility.",
        ));

        commands.push(PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check -p bevy_window --no-default-features --features libm,bevy_reflect,serialize --target {target}"
            ),
            "Please fix compiler errors in output above for bevy_state no_std compatibility.",
        ));

        commands.push(PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check -p bevy_transform --no-default-features --features bevy-support,serialize,libm --target {target}"
            ),
            "Please fix compiler errors in output above for bevy_transform no_std compatibility.",
        ));

        commands
    }
}
