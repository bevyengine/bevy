//! Utilities for state examples.
#![allow(dead_code)]

use bevy::prelude::*;

/// Entities marked with this component will be removed
/// when the provided no longer matches the world state.
///
/// To enable this feature, register the
/// [`clear_state_bound_entities`] system
/// for selected states.
#[derive(Component)]
pub struct StateBound<S: States>(pub S);

/// Removes entities marked with [`StateBound<S>`]
/// when their state no longer matches the world state.
pub fn clear_state_bound_entities<S: States>(
    state: S,
) -> impl Fn(Commands, Query<(Entity, &StateBound<S>)>) {
    move |mut commands, query| {
        for (entity, bound) in &query {
            if bound.0 == state {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

/// Logs state transitions into console.
pub fn log_transitions<S: States>(mut transitions: EventReader<StateTransitionEvent<S>>) {
    for transition in transitions.read() {
        info!(
            "Transition: {:?} => {:?}",
            transition.exited, transition.entered
        );
    }
}
