//! CI script used for Bevy.

mod ci;
mod prepare;
mod commands;

pub(crate) use self::ci::*;
pub(crate) use self::prepare::*;

fn main() {
    argh::from_env::<CI>().run();
}
