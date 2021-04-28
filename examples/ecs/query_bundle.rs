use bevy::{ecs::schedule::RunOnce, log::LogPlugin, prelude::*};

fn main() {
    App::build()
        .add_plugin(LogPlugin)
        .add_startup_system(setup.system())
        .add_system(
            query_component_without_person_bundle
                .system()
                .with_run_criteria(RunOnce::default()),
        )
        .add_system(
            query_person_bundle
                .system()
                .with_run_criteria(RunOnce::default()),
        )
        .run();
}

#[derive(Debug)]
struct Name(String);

#[derive(Debug)]
struct Age(usize);

#[derive(Debug, Bundle)]
struct PersonBundle {
    name: Name,
    age: Age,
}

/// Sets up two entities, one with a [Name] component as part of a bundle,
/// and one entity with [Name] only.
fn setup(mut commands: Commands) {
    commands.spawn().insert(Name("Steve".to_string()));

    commands.spawn().insert_bundle(PersonBundle {
        name: Name("Bob".to_string()),
        age: Age(40),
    });
}

fn query_component_without_person_bundle(query: Query<&Name>) {
    info!("Show all components");
    // this will necessarily have to print both components.
    query.iter().for_each(|x| {
        info!("{:?}", x);
    });
}
fn query_person_bundle(query: Query<&Name, WithBundle<PersonBundle>>) {
    info!("Print component initiated from bundle.");
    // this should only print `Name("Bob")`.
    query.iter().for_each(|x| {
        info!("{:?}", x);
    });
}
