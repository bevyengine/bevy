//! CI script used for Bevy.

mod ci;
mod commands;
mod json;
mod prepare;

pub use self::{ci::*, prepare::*};
use std::process::ExitCode;

fn main() -> ExitCode {
    argh::from_env::<CI>().run()
}
