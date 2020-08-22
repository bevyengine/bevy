use bevy::{prelude::*};
use bevy_net::{NetPlugin, NetProtocol, OpenSocket, SendSocket};

const REMOTE_PORT: Port = 4000;

// This example sends data to a localhost TCP server running on port 4000
// To receive data use netcat: `netcat -l 4000`
fn main() {
    App::build()
        .add_plugin(NetPlugin)
        .add_resource(SocketId::new())
        .add_startup_system(setup.system())
        .add_default_plugins()
        .add_system_to_stage(stage::UPDATE, send_data_system.system())
        .run();
}

fn setup(
    mut socket_id: ResMut<SocketId>,
    mut socket_open: ResMut<Events<OpenSocket>>,
) {
    // Open a TCP socket to localhost:REMOTE_PORT
    socket_open.send(OpenSocket {
        new_id: *socket_id,
        remote_address: SocketAddress::new(IpAddress::from([127, 0, 0, 1]), REMOTE_PORT),
        protocol: NetProtocol::Tcp,
    });
}

fn send_data_system(
    time: Res<Time>,
    mut socket_id: ResMut<SocketId>,
    mut socket_send: ResMut<Events<SendSocket>>,
) {
    // Create and send a simple message through the socket
    let message = format!("Around {} second(s) have elapsed since the app started\n", time.seconds_since_startup.ceil());
    socket_send.send(SendSocket {
        id: *socket_id,
        data: message.into_bytes(),
    });
}
