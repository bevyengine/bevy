use bevy_ecs::{event::Event, prelude::Entity};
#[cfg(feature = "reflect")]
use bevy_reflect::Reflect;

/// A [`Trigger`] emitted for entities whenever they are added as a child to an entity,
/// removed from their parent, or moved from one parent to another.
///
/// [`Trigger`]: bevy_ecs::observer::Trigger
///
/// # Example
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_hierarchy::{BuildChildren, OnParentChange};
/// #
/// let mut world = World::new();
///
/// // Add an observer to listen for hierarchy changes.
/// world.add_observer(move |trigger: Trigger<OnParentChange>| {
///     let entity = trigger.entity();
///     match trigger.event() {
///         OnParentChange::Added(parent) => {
///             println!("Entity {entity} was added as a child to {parent}");
///         }
///         OnParentChange::Removed(parent) => {
///             println!("Entity {entity} was removed from its parent {parent}");
///         }
///         OnParentChange::Moved { previous, new } => {
///             println!("Entity {entity} was moved from parent {previous} to {new}");
///         }
///     }
/// });
///
/// let parent1 = world.spawn_empty().id();
/// let parent2 = world.spawn_empty().id();
/// let child = world.spawn_empty().id();
///
/// // Triggers `OnParentChange::Added`.
/// world.entity_mut(parent1).add_child(child);
///
/// // Triggers `OnParentChange::Moved`.
/// world.entity_mut(parent2).add_child(child);
///
/// // Triggers `OnParentChange::Removed`.
/// world.entity_mut(child).remove_parent();
/// ```
#[derive(Event, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "reflect", derive(Reflect), reflect(Debug, PartialEq))]
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
