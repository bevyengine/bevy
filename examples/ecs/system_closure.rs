use bevy::{log::LogPlugin, prelude::*};

fn main() {
    // create a non-capturing closure.
    let non_capturing_closure = || {
        info!("Hello from a non-capturing closure!");
    };

    // create a capturing closure, with a Local value.
    let capturing_closure = |value: String| {
        move |mut arg: Local<String>| {
            *arg = value.clone();
            info!("Hello from a capturing closure! {:?}", arg);
        }
    };

    let outside_variable = "bar".to_string();

    App::new()
        .add_plugin(LogPlugin)
        // we can use a non-capturing closure as a system
        .add_startup_system(non_capturing_closure)
        // or we can use a capturing closure, and pass an argument to it
        .add_startup_system(capturing_closure("foo".into()))
        // we can also inline closure
        .add_startup_system(|| {
            info!("Hello from an inlined non-capturing closure!");
        })
        // or use variables outside the closure to initialize, for example, a Local variable
        .add_startup_system(move |mut arg: Local<String>| {
            *arg = outside_variable.clone();
            info!("Hello from an inlined capturing closure! {:?}", arg);
        })
        .run();
}
