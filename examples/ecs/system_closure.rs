use bevy::{log::LogPlugin, prelude::*};

fn main() {
    // create a simple closure.
    let simple_closure = || {
        info!("Hello from a simple closure!");
    };

    // create a closure, with a Local value to be initialized.
    let complex_closure = |value: String| {
        move |mut arg: Local<String>| {
            *arg = value.clone();
            info!("Hello from a complex closure! {:?}", arg);

            // 'arg' will be saved between calls.
            // you could also use an outside variable like presented in then inlined closures
        }
    };

    let outside_variable = "bar".to_string();

    App::new()
        .add_plugin(LogPlugin)
        // we can use a closure as a system
        .add_system(simple_closure)
        // or we can use a more complex closure, and pass an argument to initialize a Local variable.
        .add_system(complex_closure("foo".into()))
        // we can also inline a closure
        .add_system(|| {
            info!("Hello from an inlined closure!");
        })
        // or use variables outside a closure
        .add_system(move || {
            info!("Hello from an inlined closure that captured the 'outside_variable'! {:?}", outside_variable);
            // you can use outside_variable, or any other variables inside this closure.
            // there states will be saved.
        })
        .run();
}
