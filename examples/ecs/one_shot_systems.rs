//! Demonstrates the use of "one-shot systems", which run once when triggered.
//!
//! These can be useful to help structure your logic in a push-based fashion,
//! reducing the overhead of running extremely rarely run systems
//! and improving schedule flexibility.
//!
//! See the [`World::run_system`](bevy::ecs::World::run_system) or
//! [`World::run_system_once`](bevy::ecs::World::run_system_once) docs for more
//! details.

use bevy::{
    ecs::system::{RunSystemOnce, SystemId},
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
fn count_entities(all_entities: Query<()>) {
    dbg!(all_entities.iter().count());
}

#[derive(Component)]
struct Callback(SystemId);

#[derive(Component)]
struct Triggered;

fn setup(world: &mut World) {
    let button_pressed_id = world.register_system(button_pressed);
    world.spawn((Callback(button_pressed_id), Triggered));
    // This entity does not have a Triggered component, so its callback won't run.
    let slider_toggled_id = world.register_system(slider_toggled);
    world.spawn(Callback(slider_toggled_id));
    world.run_system_once(count_entities);
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
        commands.run_system(callback.0);
    }
}
