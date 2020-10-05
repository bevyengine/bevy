#[cfg(target_arch = "wasm32")]
extern crate console_error_panic_hook;

use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use std::time::Duration;
use futures_lite::future;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use crate::run_app;
    use wasm_bindgen::prelude::*;

    // Prevent `wasm_bindgen` from autostarting main on all spawned threads
    #[wasm_bindgen(start)]
    pub fn dummy_main() {
    }

    // Export explicit run function to start main
    #[wasm_bindgen]
    pub async fn run() {
        console_log::init_with_level(log::Level::Trace).expect("cannot initialize console_log");
        console_error_panic_hook::set_once();
        run_app().await;
    }
}

fn main() {
    future::block_on(run_app());
}

async fn run_app() {
    App::build()
        .add_plugin(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .add_startup_system(hello_world_system.system())
        .add_system(counter.system())
        .add_system(test1.system())
        .add_system(test2.system())
        .add_system(test3.system())
        .run().await;
}

fn hello_world_system() {
    log::info!("hello wasm");
}

fn counter(mut state: Local<CounterState>) {
    if state.count % 60 == 0 {
        log::info!("counter system: {}", state.count);
    }
    state.count += 1;
}

fn test1() {
}
fn test2() {
}
fn test3() {
}

#[derive(Default)]
struct CounterState {
    count: u32,
}
