use bevy::{prelude::*, log::LogPlugin};

fn main() {
    let f = |value: String| {
        move |mut arg: Local<String>| {
            *arg = value.clone();
            info!("MyWrapper is : {:?}", arg);
        }
    };

    App::new()
        .add_plugin(LogPlugin)
        .add_startup_system(f("Hello, World!".into()))
        .run();
}
