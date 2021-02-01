use bevy::{log::LogPlugin, prelude::*};

fn main() {
    App::build()
        .add_plugin(LogPlugin::default())
        .add_system(hello_wasm_system.system())
        .run();
}

fn hello_wasm_system() {
    info!("hello wasm");
}
