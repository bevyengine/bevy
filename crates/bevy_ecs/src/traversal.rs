//! A trait for components that let you traverse the ECS.

use crate::{
    component::{Component, StorageType},
    entity::Entity,
};

/// A component that can point to another entity, and which can be used to define a path through the ECS.
///
/// Traversals are used to [specify the direction] of [event propagation] in [observers]. By default,
/// events use the [`TraverseNone`] placeholder component, which cannot actually be created or added to
/// an entity and so never causes traversal.
///
/// The implementer is responsible for ensuring that `Traversal::next` cannot produce infinite loops.
///
/// [specify the direction]: crate::event::Event::Traverse
/// [event propagation]: crate::observer::Trigger::propagate
/// [observers]: crate::observer::Observer
pub trait Traversal: Component {
    /// Returns the next entity to visit.
    fn next(&self) -> Option<Entity>;
}

/// A traversal component that doesn't traverse anything. Used to provide a default traversal
/// implementation for events.
///
/// It is not possible to actually construct an instance of this component.
pub enum TraverseNone {}

impl Traversal for TraverseNone {
    #[inline(always)]
    fn next(&self) -> Option<Entity> {
        None
    }
}

impl Component for TraverseNone {
    const STORAGE_TYPE: StorageType = StorageType::Table;
}
