//! Shows how to create systems that run every fixed timestep, rather than every tick.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // this system will run once every update (it should match your screen's refresh rate)
        .add_systems(Update, frame_update)
        // add our system to the fixed timestep schedule
        .add_systems(FixedUpdate, fixed_update)
        // configure our fixed timestep schedule to run twice a second
        .insert_resource(Time::<Fixed>::from_seconds(0.5))
        .run();
}

fn frame_update(mut last_time: Local<f32>, time: Res<Time>) {
    // Default `Time` is `Time<Virtual>` here
    info!(
        "time since last frame_update: {}",
        time.elapsed_seconds() - *last_time
    );
    *last_time = time.elapsed_seconds();
}

fn fixed_update(mut last_time: Local<f32>, time: Res<Time>, fixed_time: Res<Time<Fixed>>) {
    // Default `Time`is `Time<Fixed>` here
    info!(
        "time since last fixed_update: {}\n",
        time.elapsed_seconds() - *last_time
    );

    info!("fixed timestep: {}\n", time.delta_seconds());
    // If we want to see the overstep, we need to access `Time<Fixed>` specifically
    info!(
        "time accrued toward next fixed_update: {}\n",
        fixed_time.overstep().as_secs_f32()
    );
    *last_time = time.elapsed_seconds();
}
