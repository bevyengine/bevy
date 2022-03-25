use bevy::{prelude::*, log::LogPlugin};

#[derive(Default)]
struct MyConfig(u32);


fn configure_local_example(value: String) -> impl FnMut(Local<String>) {
    move |mut arg| {
        *arg = value.clone();
        info!("MyWrapper is : {:?}", arg);
    }
}

fn main() {   
    App::new()
        .add_plugin(LogPlugin)
        .add_startup_system(configure_local_example("Hello, World!".into()))
        .run();
}
