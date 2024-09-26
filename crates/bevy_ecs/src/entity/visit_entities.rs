use std::iter;

use crate::entity::Entity;

/// Apply an operation to all entities in a container.
///
/// This is implemented by default for types that implement [`IterEntities`] or
/// where `&T` and `&mut T` both implement [`IntoIterator`].
///
/// It may be useful to implement directly for types that can't produce an
/// iterator for lifetime reasons, such as those involving internal mutexes.
pub trait VisitEntities {
    /// Apply an operation to all contained entities.
    fn visit_entities<F: FnMut(Entity)>(&self, f: F);
    /// Apply an operation to mutable references to all entities.
    fn visit_entities_mut<F: FnMut(&mut Entity)>(&mut self, f: F);
}

impl<T> VisitEntities for T
where
    T: IterEntities,
{
    fn visit_entities<F: FnMut(Entity)>(&self, f: F) {
        self.iter_entities().for_each(f);
    }
    fn visit_entities_mut<F: FnMut(&mut Entity)>(&mut self, f: F) {
        self.iter_entities_mut().for_each(f);
    }
}

/// Produce an iterator over all contained entities.
///
/// This is implemented by default for types  where `&T` and `&mut T` both implement
/// [`IntoIterator`].
///
/// It may be useful to implement directly for types that can't produce an
/// iterator for lifetime reasons, such as those involving internal mutexes.
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
    /// Get an iterator over mutable references to contained entities.
    fn iter_entities_mut(&mut self) -> impl Iterator<Item = &mut Entity>;
}

impl<T> IterEntities for T
where
    for<'a> &'a T: IntoIterator<Item = &'a Entity>,
    for<'a> &'a mut T: IntoIterator<Item = &'a mut Entity>,
{
    fn iter_entities_mut(&mut self) -> impl Iterator<Item = &mut Entity> {
        self.into_iter()
    }
    fn iter_entities(&self) -> impl Iterator<Item = Entity> {
        self.into_iter().copied()
    }
}

impl IterEntities for Entity {
    fn iter_entities_mut(&mut self) -> impl Iterator<Item = &mut Entity> {
        iter::once(self)
    }

    fn iter_entities(&self) -> impl Iterator<Item = Entity> {
        iter::once(*self)
    }
}
