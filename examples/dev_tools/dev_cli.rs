//! Show how to use DevCommands, DevTools and cli dev console

use bevy::prelude::*;
use bevy::dev_tools::console_reader_plugin::ConsoleReaderPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ConsoleReaderPlugin)
        .run();
}