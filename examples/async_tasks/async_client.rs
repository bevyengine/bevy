//! async_client - a very basic example of how to use the async_source_external_thread 
//! example (https://github.com/bevyengine/bevy/blob/main/examples/async_tasks/external_source_external_thread.rs)
//! to implement a server/client model. This is the client, which will start up, send an initial packet to the server,
//! spawn a thread that listens for incoming connections and parses incoming connections and passes them into the
//! crossbeam_channel. Another system reads from that channel and writes to the Bevy event queue, which is then read from 
//! and parsed to, in this case, spawn a rectangle.


use std::io::prelude::*;
use std::thread;
use std::net::{TcpStream, TcpListener};
use bevy::{prelude::*, sprite::MaterialMesh2dBundle,tasks::{AsyncComputeTaskPool, Task},};
use crossbeam_channel::{bounded, Receiver};

#[derive(Component)]
struct ComputeSpawn(Task<SpriteBundle>);

#[derive(Resource, Deref)]
struct StreamRec(Receiver<String>);
struct StreamEvent(String);

// Initially connect to the server
fn connect(){
    let mut stream = TcpStream::connect("127.0.0.1:7878").unwrap();
    stream.write(b"yeet");
}

// A separate function to parse string input
fn handleStream(mut stream : TcpStream) -> String{
    let mut buffer = String::new();
    stream.read_to_string(&mut buffer);
    println!("Received {}",buffer);
    if buffer == "spawn"{
       "spawn".to_string() 

    }else{
        "".to_string()
    }

}
// The function that actually polls the events queue and, if it finds the "spawn" 
// command in the event reader queue, will actually spawn the rectangle
fn spawner(mut commands : Commands,
           mut reader : EventReader<StreamEvent>){
    let events : Vec<_> = reader.iter().collect();
    for event in events{
        println!("Pulled from event system: {}",event.0);
        if event.0 == "spawn"{
            let rect = SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.25, 0.25, 0.75),
                    custom_size: Some(Vec2::new(50.0, 100.0)),
                    ..default()
                },
                ..default()
            };
            commands.spawn(rect); 
        }

    }

}

// Basically, this bridges from the channel comms to the Bevy App.
// This function reads from the channel reader and adds events to the 
// App Events queue (which is read by the spawner() function)
fn readstream(receiver: Res<StreamRec>, mut events: EventWriter<StreamEvent>){
    for from_str in receiver.try_iter(){
        println!("Received an event from the StreamReceiver: {:?}",from_str);
        events.send(StreamEvent(from_str));
    }
}
// Spawns a camera and then spawns a thread that listens on the :8080 port 
// for any and all connections. If it receives the connection, it passes
// the stream to the handleStream() function, which parses it and returns
// a stream that is then added to the channel reader, which is polled by 
// readstream(), which adds the event to the Bevy events queue
fn setup(mut commands : Commands){
    commands.spawn(Camera2dBundle::default());
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    println!("In listener...");
    connect(); 
    let (tx, rx) = bounded::<String>(10);
    std::thread::spawn(move || loop {
        println!("In thread...");
        for stream in listener.incoming(){
            println!("Received stream!");
            match stream {
                Ok(stream) => {
                    
                    let res = handleStream(stream);
                    println!("Received {} from handleStream",res);
                    tx.send(res);
                }
                Err(e) => {}

            }

        }
    });
    commands.insert_resource(StreamRec(rx));
     
}


fn main() {
    App::new()
        // This is important: StreamEvent needs to be on the App object to start receiving events
        .add_event::<StreamEvent>()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(readstream)
        .add_system(spawner)
        .run()
}
