use bevy_ecs::prelude::Entity;

/// A [`Event`] that is fired whenever there is a change in the world's
/// hierarchy.
///
/// [`Event`]: bevy_ecs::event::Event
#[derive(Debug, Clone)]
pub enum HierarchyEvent {
    /// Fired whenever an [`Entity`] is added as a child to a new parent.
    ChildAdded {
        /// The child that added
        child: Entity,
        /// The parent the child was added to
        parent: Entity,
    },
    /// Fired whenever an child [`Entity`] is removed from is parent.
    ChildRemoved {
        /// The child that removed
        child: Entity,
        /// The parent the child was removed from
        parent: Entity,
    },
    /// Fired whenever an child [`Entity`] is moved to a new parent.
    ChildMoved {
        /// The child that moved
        child: Entity,
        /// The parent the child was removed from
        previous_parent: Entity,
        /// The parent the child was added to
        new_parent: Entity,
    },
}
