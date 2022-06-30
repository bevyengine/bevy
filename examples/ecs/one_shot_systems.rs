//! Demonstrates the use of "one-shot systems", which run once when triggered
//!
//! These can be useful to help structure your logic in a push-based fashion,
//! reducing the overhead of running extremely rarely run systems
//! and improving schedule flexibility.
//!
//! See the [`SystemRegistry`](bevy::ecs::SystemRegistry) docs for more details.

use bevy::{
    ecs::system::{SystemId, SystemRegistry},
    prelude::*,
};

fn main() {
    App::new()
        .add_systems(Startup, (count_entities, setup))
        .add_systems(PostUpdate, count_entities)
        .add_systems(Update, evaluate_callbacks)
        .run();
}

// Any ordinary system can be run via commands.run_system or world.run_system.
//
// Chained systems, exclusive systems and systems which themselves run systems cannot be called in this way.
fn count_entities(all_entities: Query<()>) {
    dbg!(all_entities.iter().count());
}

#[derive(Component)]
struct Callback(SystemId);

#[derive(Component)]
struct Triggered;

fn setup(mut system_registry: ResMut<SystemRegistry>, mut commands: Commands) {
    commands.spawn((
        Callback(system_registry.register(button_pressed)),
        Triggered,
    ));
    // This entity does not have a Triggered component, so its callback won't run.
    commands.spawn(Callback(system_registry.register(slider_toggled)));
    commands.run_system(count_entities);
}

fn button_pressed() {
    println!("A button was pressed!");
}

fn slider_toggled() {
    println!("A slider was toggled!");
}

/// Runs the systems associated with each `Callback` component if the entity also has a Triggered component.
///
/// This could be done in an exclusive system rather than using `Commands` if preferred.
fn evaluate_callbacks(query: Query<&Callback, With<Triggered>>, mut commands: Commands) {
    for callback in query.iter() {
        commands.run_system_by_id(callback.0);
    }
}
