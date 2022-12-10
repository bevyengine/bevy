//! async_server - a very basic example of how to use the async_source_external_thread 
//! example (https://github.com/bevyengine/bevy/blob/main/examples/async_tasks/external_source_external_thread.rs)
//! to implement a server/client model. This server example creates a plugin that initializes a listener that sends incoming streams
//! to a parsing function, which will parse the sent stream from the client, compare it to a hardcoded value and send a response to the 
//! client


use bevy::prelude::*;
use bevy::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::io::{Read, Write};
pub struct ServerPlugin;

// Sets up a listener on port 7878
fn streamListener(){
    println!("Server initialized!");
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        thread::spawn(|| {
            handleStream(stream);
        });
        println!("Connection established!");
    
    }
}
// Parses the incoming stream and checks it against hardcoded values before sending something back
fn handleStream(mut stream : TcpStream){
    // With the way this is set up now, the initial ("yeet") packet essentially comes in from a (dynamic) random port.
    // Instead of this, we want to parse out the IP and send the response back to the port the client will be listening
    // on (:8080)
    let addr = stream.peer_addr().unwrap().to_string(); 
    let spl : Vec<&str>= addr.split(":").collect();
    let ip = spl[0];
    println!("IP: {:?}",ip);
    // Reads the stream in to a String buffer
    let mut buffer = String::new();
    stream.read_to_string(&mut buffer);
    // Checks buffer string against hardcoded value
    if(buffer == "yeet".to_string()){
        println!("Yeet command received");
        // Note: will panic if the client sends the yeet packet and then kills itself before the server can connect
        let mut stream = TcpStream::connect(ip.to_owned()+":8080").unwrap();
        stream.write(b"spawn");
    }
    
}
impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        // add things to your app here
        app.add_system(streamListener);    
    }
    
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(ServerPlugin)
        .run();
}
