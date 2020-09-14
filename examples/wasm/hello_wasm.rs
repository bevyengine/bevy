extern crate console_error_panic_hook;
use bevy::prelude::*;
use std::panic;

fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));

    App::build().add_system(hello_wasm_system.system()).run();
}

fn hello_wasm_system() {
    log::info!("hello wasm");
}
