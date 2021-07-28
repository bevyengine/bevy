use bevy::{
    app::{ScheduleRunnerPlugin, ScheduleRunnerSettings},
    log::LogPlugin,
    prelude::*,
    utils::Duration,
};

fn main() {
    App::new()
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .add_plugin(ScheduleRunnerPlugin::default())
        .add_plugin(LogPlugin::default())
        .add_startup_system(hello_world_system)
        .add_system(counter)
        .run();
}

fn hello_world_system() {
    info!("hello wasm");
}

fn counter(mut state: Local<CounterState>) {
    if state.count % 60 == 0 {
        info!("counter system: {}", state.count);
    }
    state.count += 1;
}

#[derive(Default)]
struct CounterState {
    count: u32,
}
