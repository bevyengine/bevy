#![allow(clippy::result_unit_err)]

//! CI script used for Bevy.

mod ci;
mod commands;
mod json;

pub use self::ci::*;
use std::process::ExitCode;

fn main() -> ExitCode {
    match argh::from_env::<CI>().run() {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
