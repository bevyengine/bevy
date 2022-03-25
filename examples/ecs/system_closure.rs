use bevy::{log::LogPlugin, prelude::*};

fn main() {
    let f = |value: String| {
        move |mut arg: Local<String>| {
            *arg = value.clone();
            info!("The value is : {:?}", arg);
        }
    };

    App::new()
        .add_plugin(LogPlugin)
        .add_startup_system(f("Hello, Closure 1!".into()))
        .add_startup_system(Box::new(|value: String| {
            move |mut arg: Local<String>| {
                *arg = value.clone();
                info!("The value is : {:?}", arg);
            }
        })("Hello, Closure 2!".to_string()))
        .run();
}
