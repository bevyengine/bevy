//! Tools for debugging states.

use bevy_ecs::event::EventReader;
use bevy_state::state::{StateTransitionEvent, States};
use bevy_utils::tracing::info;

/// Logs state transitions into console.
///
/// This system is provided to make debugging easier by tracking state changes.
pub fn log_transitions<S: States>(mut transitions: EventReader<StateTransitionEvent<S>>) {
    // State internals can generate at most one event (of type) per frame.
    let Some(transition) = transitions.read().last() else {
        return;
    };
    let name = std::any::type_name::<S>();
    let StateTransitionEvent { exited, entered } = transition;
    info!("{} transition: {:?} => {:?}", name, exited, entered);
}
