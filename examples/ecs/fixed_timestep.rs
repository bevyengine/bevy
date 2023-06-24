//! Shows how to create systems that run every fixed timestep, rather than every tick.

use bevy::prelude::*;
use bevy::time::TimeContext;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // set fixed timestep to run systems ten times a second
        .insert_resource(FixedTimestep::from_hz(10.0))
        // this system will run every update (this should match the screen refresh rate)
        .add_systems(Update, frame_update)
        // this system will run ten times a second
        .add_systems(FixedUpdate, fixed_update)
        .run();
}

fn frame_update(mut last_time: Local<f32>, time: Res<Time>, real_time: Res<RealTime>) {
    info!(
        "time since last frame_update: {}",
        real_time.elapsed_seconds() - *last_time
    );

    assert!(matches!(time.context(), TimeContext::Update));
    *last_time = real_time.elapsed_seconds();
}

fn fixed_update(
    mut last_time: Local<f32>,
    time: Res<Time>,
    real_time: Res<RealTime>,
    fixed_timestep: Res<FixedTimestep>,
) {
    assert!(matches!(time.context(), TimeContext::FixedUpdate));
    assert_eq!(time.delta(), fixed_timestep.size());

    info!("fixed timestep: {}\n", time.delta_seconds());
    info!(
        "time since last fixed_update: {}\n",
        real_time.elapsed_seconds() - *last_time
    );

    info!(
        "time accrued toward next fixed_update: {}\n",
        fixed_timestep.overstep().as_secs_f32()
    );
    *last_time = real_time.elapsed_seconds();
}
