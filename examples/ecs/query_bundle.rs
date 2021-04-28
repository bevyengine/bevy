use bevy::{log::LogPlugin, prelude::*};

fn main() {
    App::build()
        .add_plugin(LogPlugin)
        .add_startup_system(setup.system())
        .add_system(log_names.system().label(LogNamesSystem))
        .add_system(log_person_bundles.system().after(LogNamesSystem))
        .run();
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
struct LogNamesSystem;

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

fn log_names(query: Query<&Name>) {
    info!("Log all entities with `Name` component");
    // this will necessarily have to print both components.
    for name in query.iter() {
        info!("{:?}", name);
    }
}
fn log_person_bundles(query: Query<&Name, WithBundle<PersonBundle>>) {
    info!("Log `Name` components from entities that have all components in `PersonBundle`.");
    // this should only print `Name("Bob")`.
    for name in query.iter() {
        info!("{:?}", name);
    }
}
