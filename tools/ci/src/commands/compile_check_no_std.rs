use crate::{Flag, Prepare, PreparedCommand};
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
    fn prepare<'a>(&self, sh: &'a xshell::Shell, _flags: Flag) -> Vec<PreparedCommand<'a>> {
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
                "cargo check -p bevy_ptr --no-default-features --target {target}"
            ),
            "Please fix compiler errors in output above for bevy_ptr no_std compatibility.",
        ));

        commands.push(PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check -p bevy_utils --no-default-features --target {target}"
            ),
            "Please fix compiler errors in output above for bevy_utils no_std compatibility.",
        ));

        commands.push(PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check -p bevy_mikktspace --no-default-features --features libm --target {target}"
            ),
            "Please fix compiler errors in output above for bevy_mikktspace no_std compatibility.",
        ));

        commands
    }
}
