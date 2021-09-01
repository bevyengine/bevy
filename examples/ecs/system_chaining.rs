use anyhow::Result;
use bevy::prelude::*;

fn main() {
    App::new()
        .insert_resource(Message("42".to_string()))
        .add_system(parse_message_system.chain(handler_system))
        .run();
}

struct Message(String);

// this system produces a Result<usize> output by trying to parse the Message resource
fn parse_message_system(message: Res<Message>) -> Result<usize> {
    Ok(message.0.parse::<usize>()?)
}

// This system takes a Result<usize> input and either prints the parsed value or the error message
// Try changing the Message resource to something that isn't an integer. You should see the error
// message printed.
fn handler_system(In(result): In<Result<usize>>) {
    match result {
        Ok(value) => println!("parsed message: {}", value),
        Err(err) => println!("encountered an error: {:?}", err),
    }
}
