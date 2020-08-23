use bevy_app::{
    prelude::{EventReader, Events},
};
use bevy_ecs::{Local, Res, ResMut};

use crate::{CloseListener, CreateListener, ListenerClosed, ListenerConnected, ListenerCreated, ListenerError, Listeners, NetError, SocketClosed, SocketReceive, SocketAddress, IpAddress};
use crate::common::NetProtocol;
use crate::event::{CloseSocket, OpenSocket, SendSocket, SocketError, SocketOpened, SocketSent};
use crate::listener::{Listener, ListenerTcp, ListenerUdp};
use crate::socket::{Socket, SocketTcp, SocketUdp};
use crate::sockets::Sockets;

// Socket systems

// Handle socket open
pub fn open_socket_events_system(
    mut sockets: ResMut<Sockets>,
    mut state: Local<EventReader<OpenSocket>>,
    open_socket_events: Res<Events<OpenSocket>>,
    mut socket_opened_events: ResMut<Events<SocketOpened>>,
    mut socket_error_events: ResMut<Events<SocketError>>,
) {
    for open_socket_event in state.iter(&open_socket_events) {
        let sock = match open_socket_event.protocol {
            NetProtocol::Udp => Result::<Box<dyn Socket>, ()>::from(SocketUdp::connect(open_socket_event.remote_address,
                                                                                       Some(open_socket_event.new_id),
                                                                                       open_socket_event.port,
                                                                                       None).map(|s| Box::new(s) as Box<dyn Socket>)),
            NetProtocol::Tcp => Result::<Box<dyn Socket>, ()>::from(SocketTcp::connect(open_socket_event.remote_address,
                                                                                       Some(open_socket_event.new_id),
                                                                                       open_socket_event.port,
                                                                                       None).map(|s| Box::new(s) as Box<dyn Socket>)),
        };

        if let Ok(socket) = sock {
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
pub fn send_socket_events_system(
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
pub fn socket_receive_system(
    mut sockets: ResMut<Sockets>,
    mut socket_receive_events: ResMut<Events<SocketReceive>>,
    mut socket_error_events: ResMut<Events<SocketError>>,
) {
    for socket in sockets.iter_mut() {
        if let Ok(data) = socket.receive() {
            socket_receive_events.send(SocketReceive {
                id: socket.get_id(),
                rx_data: data,
            });
        } else {
            socket_error_events.send(SocketError {
                id: socket.get_id(),
                err: NetError::ReceiveError,
            });
        }

        // Check for connection errors
        if let Err(_) = socket.check_err() {
            socket_error_events.send(SocketError {
                id: socket.get_id(),
                err: NetError::UnknownError,
            });
        }
    }
}

// Close socket connections
pub fn close_socket_events_system(
    mut state: Local<EventReader<CloseSocket>>,
    mut sockets: ResMut<Sockets>,
    close_socket_events: Res<Events<CloseSocket>>,
    mut socket_closed_events: ResMut<Events<SocketClosed>>,
    mut socket_error_events: ResMut<Events<SocketError>>,
) {
    for close_socket_event in state.iter(&close_socket_events)
    {
        let socket = sockets.get_mut(close_socket_event.id)
            .expect("Non-existent socket ID");
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

// Listener systems
// Todo - Maybe add methods for UDP receiving/sending without a new socket

// Handle listener creation
pub fn create_listener_events_system(
    mut listeners: ResMut<Listeners>,
    mut state: Local<EventReader<CreateListener>>,
    create_listener_events: Res<Events<CreateListener>>,
    mut listener_created_events: ResMut<Events<ListenerCreated>>,
    mut listener_error_events: ResMut<Events<ListenerError>>,
) {
    for create_listener_event in state.iter(&create_listener_events) {
        let list = match create_listener_event.protocol {
            NetProtocol::Udp => Result::<Box<dyn Listener>, ()>::from(ListenerUdp::listen(create_listener_event.port,
                                                                                          Some(create_listener_event.new_id))
                .map(|l| Box::new(l) as Box<dyn Listener>)),
            NetProtocol::Tcp => Result::<Box<dyn Listener>, ()>::from(ListenerTcp::listen(create_listener_event.port,
                                                                                          Some(create_listener_event.new_id))
                .map(|l| Box::new(l) as Box<dyn Listener>)),
        };

        if let Ok(listener) = list {
            listeners.add(listener);
            listener_created_events.send(ListenerCreated {
                id: create_listener_event.new_id
            });
        } else {
            listener_error_events.send(ListenerError {
                id: create_listener_event.new_id,
                err: NetError::OpenError,
            });
        }
    }
}

// Handle listener connections
pub fn listener_connection_system(
    mut listeners: ResMut<Listeners>,
    mut sockets: ResMut<Sockets>,
    mut listener_connected_events: ResMut<Events<ListenerConnected>>,
    mut listener_error_events: ResMut<Events<ListenerError>>,
) {
    for listener in listeners.iter_mut() {
        if let Ok(data) = listener.check_incoming() {
            if let Some(socket) = data {
                let socket_id = socket.get_id();
                let remote = socket.get_remote_address()
                    .unwrap_or(SocketAddress::new(IpAddress::from([0, 0, 0, 0]), 0));
                sockets.add(socket);
                listener_connected_events.send(ListenerConnected {
                    id: listener.get_id(),
                    socket_id: socket_id,
                    socket_address: remote
                });
            }
        } else {
            listener_error_events.send(ListenerError {
                id: listener.get_id(),
                err: NetError::AcceptError,
            });
        }

        // Check for connection errors
        if let Err(_) = listener.check_err() {
            listener_error_events.send(ListenerError {
                id: listener.get_id(),
                err: NetError::UnknownError,
            });
        }
    }
}

// Handle listener close
pub fn close_listener_events_system(
    mut listeners: ResMut<Listeners>,
    mut state: Local<EventReader<CloseListener>>,
    close_listener_events: Res<Events<CloseListener>>,
    mut listener_closed_events: ResMut<Events<ListenerClosed>>,
    mut listener_error_events: ResMut<Events<ListenerError>>,
) {
    for close_listener_event in state.iter(&close_listener_events)
    {
        let listener = listeners.get_mut(close_listener_event.id)
            .expect("Non-existent listener ID");
        if listener.close().is_ok() {
            listener_closed_events.send(ListenerClosed {
                id: close_listener_event.id
            });
        } else {
            listener_error_events.send(ListenerError {
                id: close_listener_event.id,
                err: NetError::CloseError,
            });
        }
    }
}
