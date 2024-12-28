use crate::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Check for clippy warnings and errors.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "msrv")]
pub struct MsrvCommand {}

impl Prepare for MsrvCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, _flags: Flag) -> Vec<PreparedCommand<'a>> {
        vec![
            PreparedCommand::new::<Self>(
                cmd!(sh, "cargo msrv --verify"),
                "Please fix update 'rust-version' in `Cargo.toml`.",
            ),
            PreparedCommand::new::<Self>(
                cmd!(sh, "cargo msrv --path ./crates/bevy_color --verify"),
                "Please fix update 'rust-version' in `crates/bevy_color/Cargo.toml`.",
            ),
            PreparedCommand::new::<Self>(
                cmd!(sh, "cargo msrv --path ./crates/bevy_ecs --verify"),
                "Please fix update 'rust-version' in `crates/bevy_ecs/Cargo.toml`.",
            ),
            PreparedCommand::new::<Self>(
                cmd!(sh, "cargo msrv --path ./crates/bevy_input_focus --verify"),
                "Please fix update 'rust-version' in `crates/bevy_input_focus/Cargo.toml`.",
            ),
            PreparedCommand::new::<Self>(
                cmd!(sh, "cargo msrv --path ./crates/bevy_math --verify"),
                "Please fix update 'rust-version' in `crates/bevy_math/Cargo.toml`.",
            ),
            PreparedCommand::new::<Self>(
                cmd!(sh, "cargo msrv --path ./crates/bevy_mikktspace --verify"),
                "Please fix update 'rust-version' in `crates/bevy_mikktspace/Cargo.toml`.",
            ),
            PreparedCommand::new::<Self>(
                cmd!(sh, "cargo msrv --path ./crates/bevy_ptr --verify"),
                "Please fix update 'rust-version' in `crates/bevy_ptr/Cargo.toml`.",
            ),
            PreparedCommand::new::<Self>(
                cmd!(sh, "cargo msrv --path ./crates/bevy_reflect --verify"),
                "Please fix update 'rust-version' in `crates/bevy_reflect/Cargo.toml`.",
            ),
        ]
    }
}
