//! Shows how to create systems that run every fixed timestep, rather than every tick.

use bevy::{
    prelude::*,
    time::{FixedTimestep, FixedTimesteps},
};

/// Label for our fixed time-step.
#[derive(ScheduleLabel)]
struct FixedUpdate;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // this system will run once every update (it should match your screen's refresh rate)
        .add_system(frame_update)
        // add a new schedule that runs twice a second
        .add_fixed_schedule(
            FixedTimestep::step(0.5).with_label(FixedUpdate),
            SystemStage::single_threaded().with_system(fixed_update),
        )
        .run();
}

fn frame_update(mut last_time: Local<f64>, time: Res<Time>) {
    info!("update: {}", time.seconds_since_startup() - *last_time);
    *last_time = time.seconds_since_startup();
}

fn fixed_update(mut last_time: Local<f64>, time: Res<Time>, fixed_timesteps: Res<FixedTimesteps>) {
    info!(
        "fixed_update: {}",
        time.seconds_since_startup() - *last_time,
    );

    let dbg = fixed_timesteps.get(FixedUpdate).unwrap();
    info!("  overstep_percentage: {}", dbg.overstep_percentage());

    *last_time = time.seconds_since_startup();
}
