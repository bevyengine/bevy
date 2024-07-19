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
/// Infinite loops are possible, and are not checked for. While looping can be desirable in some contexts
/// (for example, an observer that triggers itself multiple times before stopping), following an infinite
/// traversal loop without an eventual exit will can your application to hang. Each implementer of `Traversal`
/// for documenting possible looping behavior, and consumers of those implementations are responsible for
/// avoiding infinite loops in their code.
///
/// [specify the direction]: crate::event::Event::Traversal
/// [event propagation]: crate::observer::Trigger::propagate
/// [observers]: crate::observer::Observer
pub trait Traversal: Component {
    /// Returns the next entity to visit.
    fn traverse(&self) -> Option<Entity>;
}

/// A traversal component that doesn't traverse anything. Used to provide a default traversal
/// implementation for events.
///
/// It is not possible to actually construct an instance of this component.
pub enum TraverseNone {}

impl Traversal for TraverseNone {
    #[inline(always)]
    fn traverse(&self) -> Option<Entity> {
        None
    }
}

impl Component for TraverseNone {
    const STORAGE_TYPE: StorageType = StorageType::Table;
}
