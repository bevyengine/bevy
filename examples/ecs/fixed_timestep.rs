use bevy::{
    core::{FixedTimestep, FixedTimesteps},
    prelude::*,
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

fn frame_update(mut last_time: Local<f64>, time: Res<Time>) {
    info!("update: {}", time.seconds_since_startup() - *last_time);
    *last_time = time.seconds_since_startup();
}

fn fixed_update(mut last_time: Local<f64>, time: Res<Time>, fixed_timesteps: Res<FixedTimesteps>) {
    info!(
        "fixed_update: {}",
        time.seconds_since_startup() - *last_time,
    );

    let fixed_timestep = fixed_timesteps.get(LABEL).unwrap();
    info!(
        "  overstep_percentage: {}",
        fixed_timestep.overstep_percentage()
    );

    *last_time = time.seconds_since_startup();
}
