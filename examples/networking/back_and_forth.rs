///! back and forth

use bevy::prelude::*;
use bevy::net::easy_sockets::SocketManagerPlugin;

fn main() {
    App::new().add_plugins(DefaultPlugins).add_plugins(SocketManagerPlugin)
}