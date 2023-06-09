use bevy_ecs::{event::Event, prelude::Entity};

/// An [`Event`] that is fired whenever there is a change in the world's hierarchy.
///
/// [`Event`]: bevy_ecs::event::Event
#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub enum HierarchyEvent {
    /// Fired whenever an [`Entity`] is added as a child to a parent.
    ChildAdded {
        /// The child that was added
        child: Entity,
        /// The parent the child was added to
        parent: Entity,
    },
    /// Fired whenever a child [`Entity`] is removed from its parent.
    ChildRemoved {
        /// The child that was removed
        child: Entity,
        /// The parent the child was removed from
        parent: Entity,
    },
    /// Fired whenever a child [`Entity`] is moved to a new parent.
    ChildMoved {
        /// The child that was moved
        child: Entity,
        /// The parent the child was removed from
        previous_parent: Entity,
        /// The parent the child was added to
        new_parent: Entity,
    },
}
