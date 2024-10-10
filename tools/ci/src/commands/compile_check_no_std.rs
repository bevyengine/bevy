use crate::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that the project compiles for a `no_std` target.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "compile-check-no-std")]
pub struct CompileCheckNoStdCommand {}

impl Prepare for CompileCheckNoStdCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, _flags: Flag) -> Vec<PreparedCommand<'a>> {
        vec![PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check -p bevy_ptr --no-default-features --target x86_64-unknown-none"
            ),
            "Please fix compiler errors in output above for bevy_ptr no_std compatibility.",
        ),
        PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check -p bevy_utils --no-default-features --target x86_64-unknown-none"
            ),
            "Please fix compiler errors in output above for bevy_utils no_std compatibility.",
        ),
        PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo check -p bevy_mikktspace --no-default-features --features libm --target x86_64-unknown-none"
            ),
            "Please fix compiler errors in output above for bevy_mikktspace no_std compatibility.",
        )
        ]
    }
}
