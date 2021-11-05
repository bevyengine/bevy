use anyhow::Result;
use bevy::prelude::*;

/// System welding is a valuable but niche tool that allows you to pass the output from one system
/// as input into a second system on a one-to-one basis.
///
/// It is most useful for error handling, or applying similar system adaptors to existing code.
/// For more general-purpose system communication, you should probably use Events instead.
/// For system ordering, use `.before` and `.after`.
fn main() {
    App::new()
        .insert_resource(Message("42".to_string()))
        .add_system(parse_message_system.weld(handler_system))
        .run();
}

struct Message(String);

// This system produces a Result<usize> output by trying to parse the Message resource
fn parse_message_system(message: Res<Message>) -> Result<usize> {
    Ok(message.0.parse::<usize>()?)
}

// This system takes a Result<usize> input and either prints the parsed value or the error message
// Try changing the Message resource to something that isn't an integer.
// You should see the error message printed.
fn handler_system(In(result): In<Result<usize>>) {
    match result {
        Ok(value) => println!("parsed message: {}", value),
        Err(err) => println!("encountered an error: {:?}", err),
    }
}
