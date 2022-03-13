use bevy_ecs::prelude::Entity;

/// An event that is fired whenever an [`Entity`] is added as a child
/// to a new parent.
#[derive(Clone)]
pub struct ChildAdded {
    /// The child that added
    pub child: Entity,
    /// The parent the child was added to
    pub parent: Entity,
}

/// An event that is fired whenever an child [`Entity`] is removed from
/// to parent.
#[derive(Clone)]
pub struct ChildRemoved {
    /// The child that removed
    pub child: Entity,
    /// The parent the child was removed from
    pub parent: Entity,
}

/// An event that is fired whenever an child [`Entity`] is moved to
/// a new parent.
#[derive(Clone)]
pub struct ChildMoved {
    /// The child that moved
    pub child: Entity,
    /// The parent the child was removed from
    pub previous_parent: Entity,
    /// The parent the child was added to
    pub new_parent: Entity,
}
