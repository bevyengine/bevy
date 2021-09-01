use bevy::{log::LogPlugin, prelude::*};

fn main() {
    App::new()
        .add_plugin(LogPlugin::default())
        .add_system(hello_wasm_system)
        .run();
}

fn hello_wasm_system() {
    info!("hello wasm");
}
