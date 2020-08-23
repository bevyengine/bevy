use bevy::{prelude::*};
use bevy_net::{CreateListener, ListenerConnected, ListenerError, NetPlugin, NetProtocol, SendSocket};

const HOST_PORT: Port = 4000;
const LISTENER_PROTOCOL: NetProtocol = NetProtocol::Tcp;

// This example receives data from a TCP (or UDP if changed) connection on port <HOST_PORT>
// If using TCP, run the following commnand to start sending data in the terminal: `netcat localhost <HOST_PORT>`
// If using UDP, run: `netcat -u localhost <HOST_PORT>`
// To send data with netcat, simply type whatever you want to send and press the `Enter` key,
// (sends with a newline) or the combination `Ctrl` + `D` to send as-is
// Remember to run the command after running the example (especially for TCP)
fn main() {
    App::build()
        .add_plugin(NetPlugin)
        .add_resource(ListenerId::new())
        .add_startup_system(setup.system())
        .add_default_plugins()
        .add_system_to_stage(stage::UPDATE, accept_connections_system.system())
        .add_system_to_stage(stage::UPDATE, handle_error_system.system())
        .run();
}

fn setup(
    listener_id: Res<ListenerId>,
    mut listener_create: ResMut<Events<CreateListener>>,
) {
    // Open a listener on <HOST_PORT>
    listener_create.send(CreateListener {
        new_id: *listener_id,
        port: HOST_PORT,
        protocol: LISTENER_PROTOCOL,
    });
}

fn accept_connections_system(
    listener_id: Res<ListenerId>,
    mut socket_send: ResMut<Events<SendSocket>>,
    mut state: Local<EventReader<ListenerConnected>>,
    mut listener_connected_events: Res<Events<ListenerConnected>>,
) {
    // Accept connections and respond with message
    for listener_connected_event in state.iter(&listener_connected_events) {
        println!("Received a new connection from {:?}", listener_connected_event.socket_address);
        socket_send.send(SendSocket {
            id: listener_connected_event.socket_id,
            tx_data: format!("Hello!\n").into_bytes(),
        })
    }
}

fn handle_error_system(
    mut state: Local<EventReader<ListenerError>>,
    listener_error_events: Res<Events<ListenerError>>,
) {
    for listener_error_event in state.iter(&listener_error_events) {
        eprintln!("Listener error (ID: {:?}): \"{:?}\"", listener_error_event.id, listener_error_event.err);
    }
}
