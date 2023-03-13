//! Demonstrates the use of "one-shot systems", which run once when triggered.
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
        .add_startup_system(count_entities)
        .add_startup_system(setup)
        // One shot systems are interchangeable with ordinarily scheduled systems.
        // Change detection, Local and NonSend all work as expected.
        .add_system(count_entities.in_base_set(CoreSet::PostUpdate))
        // One-shot systems can be used to build complex abstractions
        // to match the needs of your design.
        // Here, we model a very simple component-linked callback architecture.
        .add_system(evaluate_callbacks)
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
        // The Callback component is defined in bevy_ecs,
        // but wrapping this (or making your own customized variant) is easy.
        // Just stored a boxed SystemLabel!
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
        // Because we don't have access to the type information of the callbacks
        // we have to use the layer of indirection provided by system labels.
        // Note that if we had registered multiple systems with the same label,
        // they would all be evaluated here.
        commands.run_system_by_id(callback.0);
    }
}
