//! Demonstrates the use of "one-shot systems", which run once when triggered
//!
//! These can be useful to help structure your logic in a push-based fashion,
//! reducing the overhead of running extremely rarely run systems
//! and improving schedule flexibility.
//!
//! See the [`SystemRegistry`](bevy::ecs::SystemRegistry) docs for more details.

use bevy::ecs::schedule::IntoSystemDescriptor;
use bevy::ecs::system::SystemTypeIdLabel;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_startup_system(setup)
        // One shot systems are interchangeable with ordinarily scheduled systems.
        // Change detection, Local and NonSend all work as expected
        .add_system_to_stage(CoreStage::PostUpdate, count_entities)
        // Registered systems can be called dynamically by their label
        // These must be registered in advance, or commands.run_system will panic
        // as no matching system was found
        .register_system(button_pressed)
        .register_system(slider_toggled)
        // One-shot systems can be used to build complex abstractions
        // to match the needs of your design.
        // Here, we model a very simple system-powered callback architecture.
        .add_system(trigger_callbacks)
        .add_system(evaluate_callbacks.after(trigger_callbacks))
        .run();
}

// Any ordinary system can be run via commands.run_system or world.run_system
//
// Chained systems, exclusive systems and systems which themselves run systems cannot be called in this way.
fn count_entities(all_entities: Query<()>) {
    dbg!(all_entities.iter().count());
}

fn setup(mut commands: Commands) {
    // commands.run_system is evaluated in sequence
    commands.run_system(count_entities); // Reports 0 entities
    commands.spawn().insert(Callback::new(button_pressed));
    commands.spawn().insert(Callback::new(slider_toggled));
    commands.run_system(count_entities); // Reports 2 entities, as the previous command will be processed by the time this has evaluated
}

fn button_pressed() {
    println!("A button was pressed!")
}

fn slider_toggled() {
    println!("A slider was toggled!")
}

// When creating abstractions built on one-shot systems,
// storing system labels is the way to go.
// Don't try to store boxed System trait objects!
#[derive(Component)]
struct Callback {
    triggered: bool,
    label: Box<dyn SystemLabel>,
}

impl Callback {
    // We can pass in a system as our function argument to automatically generate the correct label.
    // Alternatively, you can create an API that takes an explicit `SystemLabel`, and users can control
    // exactly how the system should be called.
    fn new<S: IntoSystemDescriptor<Params> + 'static, Params>(system: S) -> Self {
        Callback {
            triggered: false,
            label: Box::new(SystemTypeIdLabel::<S>::new()),
        }
    }
}

// These callbacks could easily be triggered via events, button interactions or so on
// or even stored directly in the event type
fn trigger_callbacks(query: Query<&mut Callback>) {
    for mut callback in query.iter_mut() {
        callback.triggered = true;
    }
}

/// Runs the systems associated with each Callback component
///
/// This could also be done in an exclusive system,
/// but the borrow checker is more frustrating
fn evaluate_callbacks(query: Query<&Callback>, mut commands: Commands) {
    for callback in query.iter() {
        if callback.triggered {
            // Because we don't have access to the type information of the callbacks
            // we have to use the layer of indirection provided by system labels
            // Note that if we had registered multiple systems with the same label,
            // they would all be evaluated here.
            commands.run_systems_by_boxed_label(callback.label);
        }
    }
}
