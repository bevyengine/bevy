use bevy::{core::FixedTimestep, ecs::SystemStage, prelude::*};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        // this system will run once every update (it should match your screen's refresh rate) 
        .add_system(update)
        // add a new stage that runs every two seconds
        .add_stage(
            "fixed_update",
            SystemStage::parallel()
                .with_run_criteria(FixedTimestep::step(2.0))
                .with_system(fixed_update),
        )
        .run();
}

fn update(mut last_time: Local<f64>, time: Res<Time>) {
    println!("update: {}", time.seconds_since_startup() - *last_time);
    *last_time = time.seconds_since_startup();
}

fn fixed_update(mut last_time: Local<f64>, time: Res<Time>) {
    println!("fixed_update: {}", time.seconds_since_startup() - *last_time);
    *last_time = time.seconds_since_startup();
}
