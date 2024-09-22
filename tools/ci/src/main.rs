//! CI script used for Bevy.

mod ci;
mod commands;
mod prepare;

pub use self::ci::*;
pub use self::prepare::*;

fn main() {
    argh::from_env::<CI>().run();
}
