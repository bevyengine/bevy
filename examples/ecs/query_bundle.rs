use bevy::{ecs::schedule::RunOnce, prelude::*};

fn main() {
    App::build()
        .insert_resource(bevy::app::ScheduleRunnerSettings::run_once())
        .add_plugins(MinimalPlugins)
        .add_startup_system(setup.system())
        .add_stage("diagnostic", SystemStage::single_threaded())
        .add_system_to_stage(
            "diagnostic",
            query_component_without_bundle
                .system()
                .with_run_criteria(RunOnce::default()),
        )
        .add_system_to_stage(
            "diagnostic",
            test_query_bundle
                .system()
                .with_run_criteria(RunOnce::default()),
        )
        .run();
}

#[derive(Debug)]
struct Dummy(usize);

#[derive(Debug)]
struct DummyToo(usize);

#[derive(Debug, Bundle)]
struct DummyBundle {
    dummy_component: Dummy,
    // dummy_too_component: DummyToo,
}

/// Sets up entites with [Dummy] component as part of a bundle and isolated.
fn setup(mut commands: Commands) {
    commands.spawn().insert(Dummy(111));

    commands.spawn().insert_bundle(DummyBundle {
        dummy_component: Dummy(222),
        // dummy_too_component: DummyToo(333),
    });
}

fn query_component_without_bundle(query: Query<&Dummy>) {
    println!("Show all components");
    // this will necessarily have to print both components.
    query.iter().for_each(|x| {
        dbg!(x);
    });
}
fn test_query_bundle(query: Query<&Dummy, WithBundle<DummyBundle>>) {
    println!("Print component initated from bundle.");
    // this should only print `Dummy(222)`.
    query.iter().for_each(|x| {
        dbg!(x);
    });
}
