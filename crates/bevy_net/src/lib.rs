use bevy_app::prelude::*;
use bevy_ecs::IntoQuerySystem;
pub use common::*;
pub use event::*;
pub use listeners::*;
pub use sockets::*;
use system::*;

mod common;
mod event;
mod system;
mod socket;
mod sockets;
mod listeners;
mod listener;

pub mod prelude {
    pub use crate::{listener::Listener, listeners::Listeners, socket::Socket, sockets::Sockets};
    pub use crate::common::{IpAddress, ListenerId, Port, SocketAddress, SocketId};
}

/// Adds sockets and listeners to an app
#[derive(Default)]
pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<OpenSocket>()
            .add_event::<SocketOpened>()
            .add_event::<SocketError>()
            .add_event::<SendSocket>()
            .add_event::<SocketSent>()
            .add_event::<SocketReceive>()
            .add_event::<CloseSocket>()
            .add_event::<SocketClosed>()
            .init_resource::<Sockets>()
            .add_systems_to_stage(
                bevy_app::stage::EVENT_UPDATE,
                vec![handle_open_socket_events_system.system(),
                     handle_receive_socket_events.system(),
                     handle_send_socket_events.system(),
                     close_socket_connections_system.system()])
            .add_event::<CreateListener>()
            .add_event::<ListenerCreated>()
            .add_event::<ListenerError>()
            .add_event::<ListenerConnected>()
            .add_event::<CloseListener>()
            .add_event::<ListenerClosed>()
            .init_resource::<Listeners>();
    }
}
