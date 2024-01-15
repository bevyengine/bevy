//! Shows how anonymous functions / closures can be used as systems.

use bevy::{log::LogPlugin, prelude::*};

fn main() {
    // create a simple closure.
    let simple_closure = || {
        // this is a closure that does nothing.
        info!("Hello from a simple closure!");
    };

    // create a closure, with an 'input' value.
    let complex_closure = |mut value: String| {
        move || {
            info!("Hello from a complex closure! {:?}", value);

            // we can modify the value inside the closure. this will be saved between calls.
            value = format!("{value} - updated");

            // you could also use an outside variable like presented in the inlined closures
            // info!("outside_variable! {:?}", outside_variable);
        }
    };

    let outside_variable = "bar".to_string();

    App::new()
        .add_plugins(LogPlugin::default())
        // we can use a closure as a system
        .add_systems(Update, simple_closure)
        // or we can use a more complex closure, and pass an argument to initialize a Local variable.
        .add_systems(Update, complex_closure("foo".into()))
        // we can also inline a closure
        .add_systems(Update, || {
            info!("Hello from an inlined closure!");
        })
        // or use variables outside a closure
        .add_systems(Update, move || {
            info!(
                "Hello from an inlined closure that captured the 'outside_variable'! {:?}",
                outside_variable
            );
            // you can use outside_variable, or any other variables inside this closure.
            // their states will be saved.
        })
        .run();
}
