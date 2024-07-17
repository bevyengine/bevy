use std::sync::Arc;
use bevy::net::easy_sockets::quic::{Connection, RecvStream, SendStream};
use bevy::net::easy_sockets::quic::rustls::ServerConfig;
use bevy::net::easy_sockets::SocketManagerPlugin;
///! back and forth

use bevy::prelude::*;
use bevy_internal::net::easy_sockets::Sockets;


fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SocketManagerPlugin::default());
    todo!()
}

fn set_up(
    mut connections: ResMut<Sockets<Connection>>,
    mut receive: ResMut<Sockets<RecvStream>>,
    mut send: ResMut<Sockets<SendStream>>) {
    let tls_config = ServerConfig::builder().with_no_client_auth()
}