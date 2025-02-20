//! A trait for components that let you traverse the ECS.

use crate::{entity::Entity, query::ReadOnlyQueryData, relationship::Relationship};

/// A component that can point to another entity, and which can be used to define a path through the ECS.
///
/// Traversals are used to [specify the direction] of [event propagation] in [observers].
/// The default query is `()`.
///
/// Infinite loops are possible, and are not checked for. While looping can be desirable in some contexts
/// (for example, an observer that triggers itself multiple times before stopping), following an infinite
/// traversal loop without an eventual exit will cause your application to hang. Each implementer of `Traversal`
/// for documenting possible looping behavior, and consumers of those implementations are responsible for
/// avoiding infinite loops in their code.
///
/// Traversals may be parameterized with additional data. For example, in observer event propagation, the
/// parameter `D` is the event type given in `Trigger<E>`. This allows traversal to differ depending on event
/// data.
///
/// [specify the direction]: crate::event::Event::Traversal
/// [event propagation]: crate::observer::Trigger::propagate
/// [observers]: crate::observer::Observer
pub trait Traversal<D: ?Sized>: ReadOnlyQueryData {
    /// Returns the next entity to visit.
    fn traverse(item: Self::Item<'_>, data: &D) -> Option<Entity>;
}

impl<D> Traversal<D> for () {
    fn traverse(_: Self::Item<'_>, _data: &D) -> Option<Entity> {
        None
    }
}

/// This provides generalized hierarchy traversal for use in [event propagation].
///
/// # Warning
///
/// Traversing in a loop could result in infinite loops for relationship graphs with loops.
///
/// [event propagation]: crate::observer::Trigger::propagate
impl<R: Relationship, D> Traversal<D> for &R {
    fn traverse(item: Self::Item<'_>, _data: &D) -> Option<Entity> {
        Some(item.get())
    }
}
