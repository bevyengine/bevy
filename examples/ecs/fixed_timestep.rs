//! Shows how to create systems that run every fixed timestep, rather than every tick.

use bevy::{
    prelude::*,
    time::{FixedTimestep, FixedTimesteps},
};

const LABEL: &str = "my_fixed_timestep";

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
struct FixedUpdateStage;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // this system will run once every update (it should match your screen's refresh rate)
        .add_system(frame_update)
        // add a new stage that runs twice a second
        .add_stage_after(
            CoreStage::Update,
            FixedUpdateStage,
            SystemStage::parallel()
                .with_run_criteria(
                    FixedTimestep::step(0.5)
                        // labels are optional. they provide a way to access the current
                        // FixedTimestep state from within a system
                        .with_label(LABEL),
                )
                .with_system(fixed_update),
        )
        .run();
}

fn frame_update(mut last_time: Local<f32>, time: Res<Time>) {
    info!(
        "time since last frame_update: {}",
        time.raw_elapsed_seconds() - *last_time
    );
    *last_time = time.raw_elapsed_seconds();
}

fn fixed_update(mut last_time: Local<f32>, time: Res<Time>, fixed_timesteps: Res<FixedTimesteps>) {
    info!(
        "time since last fixed_update: {}\n",
        time.raw_elapsed_seconds() - *last_time
    );

    let state = fixed_timesteps.get(LABEL).unwrap();

    info!("fixed timestep: {}\n", 0.5);
    info!(
        "time accrued toward next fixed_update: {}\n",
        state.accumulator()
    );
    info!(
        "time accrued toward next fixed_update (% of timestep): {}",
        state.overstep_percentage()
    );
    *last_time = time.raw_elapsed_seconds();
}
