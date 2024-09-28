pub use bevy_ecs_macros::IterEntities;

use core::iter;

use crate::entity::Entity;

/// Apply an operation to all entities in a container.
///
/// This is implemented by default for types that implement [`IterEntities`].
///
/// It may be useful to implement directly for types that can't produce an
/// iterator for lifetime reasons, such as those involving internal mutexes.
pub trait VisitEntities {
    /// Apply an operation to all contained entities.
    fn visit_entities<F: FnMut(Entity)>(&self, f: F);
}

impl<T> VisitEntities for T
where
    T: IterEntities,
{
    fn visit_entities<F: FnMut(Entity)>(&self, f: F) {
        self.iter_entities().for_each(f);
    }
}

/// Produce an iterator over all contained entities.
///
/// This is implemented by default for types  where `&T` implements
/// [`IntoIterator`].
///
/// This trait is derivable for structs via `#[derive(IterEntities)]`. Fields
/// not containing entities can be ignored with `#[iter_entities(ignore)]`.
///
/// # Example
///
/// ```rust
/// # use bevy_ecs::entity::{Entity, IterEntities};
/// # use bevy_utils::hashbrown::HashSet;
/// #[derive(IterEntities)]
/// struct MyEntities {
///     lots: Vec<Entity>,
///     one: Entity,
///     maybe: Option<Entity>,
///     #[iter_entities(ignore)]
///     not_an_entity: String,
/// }
/// ```
pub trait IterEntities {
    /// Get an iterator over contained entities.
    fn iter_entities(&self) -> impl Iterator<Item = Entity>;
}

impl<T> IterEntities for T
where
    for<'a> &'a T: IntoIterator<Item = &'a Entity>,
{
    fn iter_entities(&self) -> impl Iterator<Item = Entity> {
        self.into_iter().copied()
    }
}

impl IterEntities for Entity {
    fn iter_entities(&self) -> impl Iterator<Item = Entity> {
        iter::once(*self)
    }
}
