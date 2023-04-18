//! Shows how to create systems that run every fixed timestep, rather than every tick.

use bevy::prelude::*;

const FIXED_TIMESTEP: f32 = 0.5;
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // this system will run once every update (it should match your screen's refresh rate)
        .add_systems(Update, frame_update)
        // add our system to the fixed timestep schedule
        .add_systems(FixedUpdate, fixed_update)
        // configure our fixed timestep schedule to run twice a second
        .insert_resource(FixedTime::new_from_secs(FIXED_TIMESTEP))
        .run();
}

fn frame_update(mut last_time: Local<f32>, time: Res<Time>) {
    info!(
        "time since last frame_update: {}",
        time.raw_elapsed_seconds() - *last_time
    );
    *last_time = time.raw_elapsed_seconds();
}

fn fixed_update(mut last_time: Local<f32>, time: Res<Time>, fixed_time: Res<FixedTime>) {
    info!(
        "time since last fixed_update: {}\n",
        time.raw_elapsed_seconds() - *last_time
    );

    info!("fixed timestep: {}\n", FIXED_TIMESTEP);
    info!(
        "time accrued toward next fixed_update: {}\n",
        fixed_time.accumulated().as_secs_f32()
    );
    *last_time = time.raw_elapsed_seconds();
}
