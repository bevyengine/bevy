use std::error::Error;

use bevy_ecs::prelude::*;

fn main() {
    let mut world = World::new();

    let mut schedule = Schedule::new();
    schedule.add_systems((
        // This system is fallible, which means it returns a Result.
        // If it returns an error, the schedule will panic.
        // To see this happen, try changing this to `fallible_system_2`,
        // which always returns `Err`.
        fallible_system_1,
        // To prevent a fallible system from panicking, we can handle
        // the error by piping it into another system.
        fallible_system_2.pipe(error_handling_system),
        // Bevy includes a number of built-in systems for handling errors.
        // These are defined in the `bevy_ecs::prelude::system_adapter` module.
        // This adapter uses the `tracing` crate to log an info event.
        // See also `warn` and `error`.
        fallible_system_2.pipe(system_adapter::info),
        // If we don't care about a system failing, we can just ignore the error
        // and try again next frame.
        fallible_system_2.pipe(system_adapter::ignore),
    ));

    schedule.run(&mut world);
}

// A system that might fail.
// A system can only be added to a schedule if it returns nothing,
// or if it returns `Result<(), Error>` with an error type that implements std::fmt::Debug.
// This system always returns `Ok`.
fn fallible_system_1() -> Result<(), Box<dyn Error>> {
    Ok(())
}

// Another fallible system. This one always returns `Err`.
fn fallible_system_2() -> Result<(), Box<dyn Error>> {
    Err("oops")?
}

// Our system that we're using to handling errors.
// Our fallible system returns a Result, so we are taking a Result as an input.
fn error_handling_system(In(result): In<Result<(), Box<dyn Error>>>) {
    // If the system didn't return an error, we can happily do nothing.
    // If it did return an error, we'll just log it and keep going.
    if let Err(error) = result {
        eprintln!("A system returned an error: {error}");
    }
}
