use bevy_app::{
    AppExit,
    prelude::{EventReader, Events},
};
use bevy_ecs::{Local, Res, ResMut, Resources};

use crate::{NetError, SocketClosed, SocketReceive};
use crate::common::NetProtocol;
use crate::event::{CloseSocket, OpenSocket, SendSocket, SocketError, SocketOpened, SocketSent};

use super::socket::Socket;
use super::sockets::Sockets;

// Handle socket open
pub fn handle_open_socket_events_system(
    mut sockets: ResMut<Sockets>,
    mut state: Local<EventReader<OpenSocket>>,
    open_socket_events: Res<Events<OpenSocket>>,
    mut socket_opened_events: ResMut<Events<SocketOpened>>,
    mut socket_error_events: ResMut<Events<SocketError>>,
) {
    for open_socket_event in state.iter(&open_socket_events) {
        if let Ok(socket) = Socket::connect(open_socket_event.remote_address,
                                            open_socket_event.protocol.clone(),
                                            Some(open_socket_event.new_id),
                                            open_socket_event.port,
                                            None) {
            sockets.add(socket);
            socket_opened_events.send(SocketOpened {
                id: open_socket_event.new_id
            });
        } else {
            socket_error_events.send(SocketError {
                id: open_socket_event.new_id,
                err: NetError::OpenError,
            });
        }
    }
}

// Handle socket sending
pub fn handle_send_socket_events(
    mut sockets: ResMut<Sockets>,
    mut state: Local<EventReader<SendSocket>>,
    send_socket_events: Res<Events<SendSocket>>,
    mut socket_sent_events: ResMut<Events<SocketSent>>,
    mut socket_error_events: ResMut<Events<SocketError>>,
) {
    for send_socket_event in state.iter(&send_socket_events) {
        if let Ok(len) = sockets.get_mut(send_socket_event.id)
            .expect("Non-existent socket ID")
            .send(&send_socket_event.tx_data) {
            socket_sent_events.send(SocketSent {
                id: send_socket_event.id,
                len,
            });
        } else {
            socket_error_events.send(SocketError {
                id: send_socket_event.id,
                err: NetError::SendError,
            });
        }
    }
}

// Handle socket receiving
pub fn handle_receive_socket_events(
    mut sockets: ResMut<Sockets>,
    mut socket_receive_events: ResMut<Events<SocketReceive>>,
    mut socket_error_events: ResMut<Events<SocketError>>,
) {
    for socket in sockets.iter_mut() {
        if let Ok(data) = socket.recv() {
            socket_receive_events.send(SocketReceive {
                id: socket.id,
                rx_data: data,
            });
        } else {
            socket_error_events.send(SocketError {
                id: socket.id,
                err: NetError::ReceiveError,
            });
        }

        // Check for connection errors
        if let Err(_) = socket.check_err() {
            socket_error_events.send(SocketError {
                id: socket.id,
                err: NetError::UnknownError,
            });
        }
    }
}

// Close socket connections
pub fn close_socket_connections_system(
    mut state: Local<EventReader<CloseSocket>>,
    mut sockets: ResMut<Sockets>,
    close_socket_events: Res<Events<CloseSocket>>,
    mut socket_closed_events: ResMut<Events<SocketClosed>>,
    mut socket_error_events: ResMut<Events<SocketError>>,
) {
    for close_socket_event in state
        .iter(&close_socket_events)
    {
        let socket = sockets.get_mut(close_socket_event.id).unwrap();
        if socket.close().is_ok() {
            socket_closed_events.send(SocketClosed {
                id: close_socket_event.id
            });
        } else {
            socket_error_events.send(SocketError {
                id: close_socket_event.id,
                err: NetError::CloseError,
            });
        }
    }
}
