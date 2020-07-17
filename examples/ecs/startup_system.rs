use bevy::{app::ScheduleRunnerPlugin, prelude::*};

fn main() {
    App::build()
        .add_plugin(ScheduleRunnerPlugin::run_once()) // only run the app once so the printed system order is clearer
        .add_startup_system(startup_system.system())
        .add_system(normal_system.system())
        .run();
}

/// Startup systems are run exactly once when the app starts up.
/// They run right before "normal" systems run.
fn startup_system() {
    println!("startup system ran first");
}

fn normal_system() {
    println!("normal system ran second");
}
