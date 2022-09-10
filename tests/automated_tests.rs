//! This example illustrates test systems.
fn main() {
    println!("This example is special! Run it with `cargo test --example automated_tests`.");
    println!(
        "Or use `cargo test --example automated_tests -- --nocapture` to see the debug output."
    );
}

#[cfg(test)]
mod test {
    use bevy::prelude::*;

    #[test]
    fn simple_test() {
        // Setup the app with the TestPlugins – these will run fine in tests and in CI.
        // Note that many 3rd-party plugins will require DefaultPlugins, not just TestPlugins.
        let mut app = App::new();
        app.add_plugins(DefaultPlugins::for_testing())
            .add_system(increment);

        // Spawn a new entity with a Counter component, and record its ID.
        let counter_id = app.world.spawn().insert(Counter::default()).id();

        // Simulate for a 10 frames
        let num_frames = 10;
        for _ in 0..num_frames {
            app.update();
        }

        // Check that the counter was incremented 10 times.
        let count = app.world.get::<Counter>(counter_id).unwrap().counter;
        assert_eq!(count, num_frames);

        println!("Success!");
    }
    // Define a system and a component that we can use in our test.
    #[derive(Debug, Default, Component, Clone, Copy)]
    struct Counter {
        counter: u64,
    }

    /// Increment the counter every frame
    fn increment(mut query: Query<&mut Counter>) {
        for mut counter in query.iter_mut() {
            counter.counter += 1;
            println!("Counter: {}", counter.counter);
        }
    }
}
