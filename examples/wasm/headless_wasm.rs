#[cfg(target_arch = "wasm32")]
extern crate console_error_panic_hook;

use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use std::time::Duration;

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Debug).expect("cannot initialize console_log");
    }

    App::build()
        .add_plugin(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .add_startup_system(hello_world_system.system())
        .add_system(counter.system())
        .run();
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

#[derive(Default)]
struct CounterState {
    count: u32,
}
