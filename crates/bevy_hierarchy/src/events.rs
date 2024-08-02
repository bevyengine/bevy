use bevy_ecs::{event::Event, prelude::Entity};

/// A [`Trigger`] emitted whenever there is a change in the world's hierarchy.
///
/// [`Trigger`]: bevy_ecs::observer::Trigger
#[derive(Event, Clone, Debug, PartialEq)]
pub enum OnParentChange {
    /// Emitted whenever the entity is added as a child to a parent.
    Added(Entity),
    /// Emitted whenever the child entity is removed from its parent.
    Removed(Entity),
    /// Emitted whenever the child entity is moved to a new parent.
    Moved {
        /// The parent the child was removed from.
        previous: Entity,
        /// The parent the child was added to.
        new: Entity,
    },
}
