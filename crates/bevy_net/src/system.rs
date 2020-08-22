use bevy_app::{
    AppExit,
    prelude::{EventReader, Events},
};
use bevy_ecs::{Local, Res, ResMut, Resources};

use crate::event::{OpenSocket, SendSocket, SocketError, SocketOpened, SocketSent};

use super::event::CloseSocket;
use super::socket::Socket;
use super::sockets::Sockets;
use crate::common::NetProtocol;

#[derive(Default)]
pub struct CloseState {
    event_reader: EventReader<CloseSocket>,
}

// Close socket connections
pub fn close_socket_connections_system(
    mut state: Local<CloseState>,
    mut sockets: ResMut<Sockets>,
    close_socket_events: Res<Events<CloseSocket>>,
) {
    if let Some(s) = state
        .event_reader
        .iter(&close_socket_events)
        .next()
    {
        if let Some(socket) = sockets.get_mut(s.id)
        {
            socket.close();
        }
    }
}

// Handle socket creation
pub fn handle_create_socket_events_system(
    mut sockets: ResMut<Sockets>,
    mut state: Local<EventReader<OpenSocket>>,
    open_socket_events: Res<Events<OpenSocket>>,
    mut socket_opened_events: ResMut<Events<SocketOpened>>,
) {
    for open_socket_event in state.iter(&open_socket_events) {
        let mut socket = (match open_socket_event.protocol {
            NetProtocol::Udp => Socket::connect_udp(open_socket_event.remote_address, None),
            NetProtocol::Tcp => Socket::connect_tcp(open_socket_event.remote_address, None)
        }).expect("Failed to open socket");

        socket.id = open_socket_event.new_id;
        sockets.add(socket);

        socket_opened_events.send(SocketOpened {
            id: open_socket_event.new_id
        });
    }
}

// Handle socket sending
pub fn handle_send_socket_events(
    mut sockets: ResMut<Sockets>,
    mut state: Local<EventReader<SendSocket>>,
    send_socket_events: Res<Events<SendSocket>>,
    mut socket_sent_events: ResMut<Events<SocketSent>>,
) {
    for send_socket_event in state.iter(&send_socket_events) {
        let len = sockets.get_mut(send_socket_event.id)
            .unwrap()
            .send(&send_socket_event.data)
            .unwrap();
        dbg!("SEND DATA");
        dbg!(len);
        socket_sent_events.send(SocketSent {
            id: send_socket_event.id,
            len
        });
    }
}