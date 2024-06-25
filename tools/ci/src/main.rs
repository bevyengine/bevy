//! CI script used for Bevy.

mod ci;
mod commands;
mod json;
mod prepare;

use std::process::ExitCode;

pub use self::ci::*;
pub use self::prepare::*;

fn main() -> ExitCode {
    argh::from_env::<CI>().run()
}
