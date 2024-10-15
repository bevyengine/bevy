use crate::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that the project compiles for a `no_std` target.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "compile-check-no-std")]
pub struct CompileCheckNoStdCommand {
    /// the target to check against.
    /// Defaults to "x86_64-unknown-none"
    #[argh(option, default = "String::from(\"x86_64-unknown-none\")")]
    target: String,
}

impl Prepare for CompileCheckNoStdCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, _flags: Flag) -> Vec<PreparedCommand<'a>> {
        let target = self.target.as_str();
        vec![PreparedCommand::new::<Self>(cmd!(sh, "cargo check -p bevy_ptr --no-default-features --target {target}"), "Please fix compiler errors in output above for bevy_ptr no_std compatibility."), PreparedCommand::new::<Self>(cmd!(sh, "cargo check -p bevy_utils --no-default-features --target {target}"), "Please fix compiler errors in output above for bevy_utils no_std compatibility."), PreparedCommand::new::<Self>(cmd!(sh, "cargo check -p bevy_mikktspace --no-default-features --features libm --target {target}"), "Please fix compiler errors in output above for bevy_mikktspace no_std compatibility.")]
    }
}
