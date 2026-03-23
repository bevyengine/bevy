//! Tools for debugging states.

use bevy_ecs::message::MessageReader;
use bevy_state::state::{StateTransitionEvent, States};
use tracing::info;

/// Logs state transitions into console.
///
/// This system is provided to make debugging easier by tracking state changes.
pub fn log_transitions<S: States>(mut transitions: MessageReader<StateTransitionEvent<S>>) {
    // State internals can generate at most one event (of type) per frame.
    let Some(transition) = transitions.read().last() else {
        return;
    };
    let name = core::any::type_name::<S>();
    let StateTransitionEvent {
        exited,
        entered,
        allow_same_state_transitions,
    } = transition;
    let skip_text = if exited == entered && !*allow_same_state_transitions {
        " (disallowing same-state transitions)"
    } else {
        ""
    };
    info!("{name} transition: {exited:?} => {entered:?}{skip_text}");
}
