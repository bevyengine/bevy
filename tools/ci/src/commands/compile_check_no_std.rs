use std::process::Command;

use argh::FromArgs;

use super::{run_cargo_command, RustChannel};

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

impl CompileCheckNoStdCommand {
    /// TARGETS and the additional flags they require
    const TARGETS: &'static [(&'static str, &'static [&'static str])] = &[
        ("bevy_ptr", &[]),
        ("bevy_utils", &[]),
        ("bevy_mikktspace", &["--features", "libm"]),
        ("bevy_reflect", &[]),
        ("bevy_math", &["--features", "libm"]),
        ("bevy_color", &["--features", "libm"]),
        (
            "bevy_task",
            &["--features", "edge_executor,critical-section"],
        ),
        (
            "bevy_ecs",
            &[
                "--features",
                "edge_executor,critical-section,bevy_debug_stepping,bevy_reflect",
            ],
        ),
        ("bevy_app", &["--features", "bevy_reflect"]),
    ];

    pub fn run(self) -> Result<(), ()> {
        let target = self.target;

        if !self.skip_install {
            Command::new("rustup")
                .args(["target", "add", &target])
                .output()
                .map_err(|err| eprintln!("{err}"))?;
        }

        let mut flags = vec!["--no-default-features", "--target", &target];

        for &(lib, additional) in Self::TARGETS {
            flags.extend_from_slice(&["--target", lib]);
            flags.extend_from_slice(additional);

            run_cargo_command("check", RustChannel::Stable, &flags, &[])?;

            for _ in 0..(flags.len() + 2) {
                flags.pop();
            }
        }

        Ok(())
    }
}

impl Default for CompileCheckNoStdCommand {
    fn default() -> Self {
        Self {
            target: String::from("x86_64-unknown-none"),
            skip_install: false,
        }
    }
}
