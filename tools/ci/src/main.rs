//! CI script used for Bevy.

mod args;
mod ci;
mod commands;
mod prepare;

pub use self::{ci::*, prepare::*};

fn main() {
    argh::from_env::<CI>().run();
}
