//! A trait for components that let you traverse the ECS.

use crate::{entity::Entity, query::ReadOnlyQueryData};

/// A component that can point to another entity, and which can be used to define a path through the ECS.
///
/// Traversals are used to [specify the direction] of [event propagation] in [observers].
/// The default query is `()`.
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
pub trait Traversal: ReadOnlyQueryData {
    /// Returns the next entity to visit.
    fn traverse(item: Self::Item<'_>) -> Option<Entity>;
}

impl Traversal for () {
    fn traverse(_: Self::Item<'_>) -> Option<Entity> {
        None
    }
}
