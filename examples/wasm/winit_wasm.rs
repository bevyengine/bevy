extern crate console_error_panic_hook;
use bevy::prelude::*;
use std::panic;

fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Debug).expect("cannot initialize console_log");

    App::build()
        .add_default_plugins()
        .add_startup_system(hello_wasm_system.system())
        .add_system(counter.system())
        .run();
}

fn hello_wasm_system() {
    log::info!("hello wasm");
}

fn counter(mut state: Local<CounterState>, time: Res<Time>) {
    if state.count % 60 == 0 {
        log::info!(
            "tick {} @ {:?} [Δ{}]",
            state.count,
            time.time_since_startup(),
            time.delta_seconds
        );
    }
    state.count += 1;
}

#[derive(Default)]
struct CounterState {
    count: u32,
}
