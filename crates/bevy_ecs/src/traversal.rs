//! A trait for components that let you traverse the ECS.

use crate::{
    entity::Entity,
    query::{ReadOnlyQueryData, ReleaseStateQueryData},
    relationship::Relationship,
};

/// A component that can point to another entity, and which can be used to define a path through the ECS.
///
/// Traversals are used to [specify the direction] of [event propagation] in [`EntityEvent`] [observers].
/// The default query is `()`.
///
/// Infinite loops are possible, and are not checked for. While looping can be desirable in some contexts
/// (for example, an observer that triggers itself multiple times before stopping), following an infinite
/// traversal loop without an eventual exit will cause your application to hang. Each implementer of `Traversal`
/// is responsible for documenting possible looping behavior, and consumers of those implementations are responsible for
/// avoiding infinite loops in their code.
///
/// Traversals may be parameterized with additional data. For example, in observer event propagation, the
/// parameter `D` is the event type given in `On<E>`. This allows traversal to differ depending on event
/// data.
///
/// [specify the direction]: crate::event::PropagateEntityTrigger
/// [event propagation]: crate::observer::On::propagate
/// [observers]: crate::observer::Observer
/// [`EntityEvent`]: crate::event::EntityEvent
pub trait Traversal<D: ?Sized>: ReadOnlyQueryData + ReleaseStateQueryData {
    /// Returns the next entity to visit.
    fn traverse(item: Self::Item<'_, '_>, data: &D) -> Option<Entity>;
}

impl<D> Traversal<D> for () {
    fn traverse(_: Self::Item<'_, '_>, _data: &D) -> Option<Entity> {
        None
    }
}

/// This provides generalized hierarchy traversal for use in [event propagation].
///
/// # Warning
///
/// Traversing in a loop could result in infinite loops for relationship graphs with loops.
///
/// [event propagation]: crate::observer::On::propagate
impl<R: Relationship, D> Traversal<D> for &R {
    fn traverse(item: Self::Item<'_, '_>, _data: &D) -> Option<Entity> {
        Some(item.get())
    }
}
