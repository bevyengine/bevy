use bevy::{prelude::*};
use bevy_net::{NetPlugin, NetProtocol, OpenSocket, SendSocket, SocketError};

const REMOTE_PORT: Port = 4000;
const HOST_UDP_PORT: Port = 2000;
const SOCKET_PROTOCOL: NetProtocol = NetProtocol::Tcp;

// This example sends data to a localhost TCP (or UDP if changed) server running on port <REMOTE_PORT>
// If using TCP, run the following commnand to receive data in the terminal: `netcat -l <REMOTE_PORT>`
// If using UDP, run: `netcat -ul <REMOTE_PORT>`
// Remember to run the command prior to running the example (especially for TCP)
fn main() {
    App::build()
        .add_plugin(NetPlugin)
        .add_resource(SocketId::new())
        .add_startup_system(setup.system())
        .add_default_plugins()
        .add_system_to_stage(stage::UPDATE, send_data_system.system())
        .add_system_to_stage(stage::UPDATE, handle_error_system.system())
        .run();
}

fn setup(
    socket_id: Res<SocketId>,
    mut socket_open: ResMut<Events<OpenSocket>>,
) {
    // Open a socket to localhost:<REMOTE_PORT>
    socket_open.send(OpenSocket {
        new_id: *socket_id,
        remote_address: SocketAddress::new(IpAddress::from([127, 0, 0, 1]), REMOTE_PORT),
        port: if SOCKET_PROTOCOL == NetProtocol::Udp { Some(HOST_UDP_PORT) } else { None },
        protocol: SOCKET_PROTOCOL,
    });
}

fn send_data_system(
    time: Res<Time>,
    socket_id: Res<SocketId>,
    mut socket_send: ResMut<Events<SendSocket>>,
) {
    // Create and send a simple message through the socket
    let message = format!("Around {:.3} second(s) have elapsed since the app started\n", time.seconds_since_startup);
    socket_send.send(SendSocket {
        id: *socket_id,
        tx_data: message.into_bytes(),
    });
}

fn handle_error_system(
    mut state: Local<EventReader<SocketError>>,
    socket_error_events: Res<Events<SocketError>>,
) {
    for socket_error_event in state.iter(&socket_error_events) {
        eprintln!("Socket error (ID: {:?}): \"{:?}\"", socket_error_event.id, socket_error_event.err);
    }
}
