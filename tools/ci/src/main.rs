//! CI script used for Bevy.

mod commands;

fn main() {
    argh::from_env::<commands::CI>().run();
}
