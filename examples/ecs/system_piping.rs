//! Illustrates how to make a single system from multiple functions running in sequence,
//! passing the output of the first into the input of the next.

use bevy::prelude::*;
use std::num::ParseIntError;

use bevy::log::LogPlugin;
use bevy::utils::{dbg, error, info, tracing::Level, warn};

fn main() {
    App::new()
        .insert_resource(Message("42".to_string()))
        .insert_resource(OptionalWarning(Err("Got to rusty?".to_string())))
        .add_plugins(LogPlugin {
            level: Level::TRACE,
            filter: "".to_string(),
            ..default()
        })
        .add_systems(
            Update,
            (
                parse_message_system.pipe(handler_system),
                data_pipe_system.map(info),
                parse_message_system.map(dbg),
                warning_pipe_system.map(warn),
                parse_error_message_system.map(error),
                parse_message_system.map(drop),
            ),
        )
        .run();
}

#[derive(Resource, Deref)]
struct Message(String);

#[derive(Resource, Deref)]
struct OptionalWarning(Result<(), String>);

// This system produces a Result<usize> output by trying to parse the Message resource.
fn parse_message_system(message: Res<Message>) -> Result<usize, ParseIntError> {
    message.parse::<usize>()
}

// This system produces a Result<()> output by trying to parse the Message resource.
fn parse_error_message_system(message: Res<Message>) -> Result<(), ParseIntError> {
    message.parse::<usize>()?;
    Ok(())
}

// This system takes a Result<usize> input and either prints the parsed value or the error message
// Try changing the Message resource to something that isn't an integer. You should see the error
// message printed.
fn handler_system(In(result): In<Result<usize, ParseIntError>>) {
    match result {
        Ok(value) => println!("parsed message: {value}"),
        Err(err) => println!("encountered an error: {err:?}"),
    }
}

// This system produces a String output by trying to clone the String from the Message resource.
fn data_pipe_system(message: Res<Message>) -> String {
    message.0.clone()
}

// This system produces an Result<String> output by trying to extract a String from the
// OptionalWarning resource. Try changing the OptionalWarning resource to None. You should
// not see the warning message printed.
fn warning_pipe_system(message: Res<OptionalWarning>) -> Result<(), String> {
    message.0.clone()
}
