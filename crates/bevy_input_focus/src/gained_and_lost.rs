//! Contains [`FocusGained`] and [`FocusLost`] events,
//! as well as the machinery to send them when the focused entity changes.

use super::InputFocus;
use bevy_ecs::prelude::*;

/// An [`EntityEvent`] that is sent when an entity gains [`InputFocus`].
///
/// This event bubbles up the entity hierarchy, so if a child entity gains focus, its parents will also receive this event.
#[derive(EntityEvent, Debug, Clone)]
#[entity_event(auto_propagate)]
pub struct FocusGained {
    /// The entity that gained focus.
    pub entity: Entity,
}

/// An [`EntityEvent`] that is sent when an entity loses [`InputFocus`].
///
/// This event bubbles up the entity hierarchy, so if a child entity loses focus, its parents will also receive this event.
#[derive(EntityEvent, Debug, Clone)]
#[entity_event(auto_propagate)]
pub struct FocusLost {
    /// The entity that lost focus.
    pub entity: Entity,
}
