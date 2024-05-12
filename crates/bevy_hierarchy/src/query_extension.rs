use std::collections::VecDeque;

use bevy_ecs::{
    entity::Entity,
    query::{QueryData, QueryFilter, WorldQuery},
    system::Query,
};

use crate::{Children, Parent};

/// An extension trait for [`Query`] that adds hierarchy related methods.
pub trait HierarchyQueryExt<'w, 's, D: QueryData, F: QueryFilter> {
    /// Returns an [`Iterator`] of [`Entity`]s over all of `entity`s descendants.
    ///
    /// Can only be called on a [`Query`] of [`Children`] (i.e. `Query<&Children>`).
    ///
    /// Traverses the hierarchy breadth-first.
    ///
    /// # Examples
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_hierarchy::prelude::*;
    /// # #[derive(Component)]
    /// # struct Marker;
    /// fn system(query: Query<Entity, With<Marker>>, children_query: Query<&Children>) {
    ///     let entity = query.single();
    ///     for descendant in children_query.iter_descendants(entity) {
    ///         // Do something!
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    fn iter_descendants(&'w self, entity: Entity) -> DescendantIter<'w, 's, D, F>
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w Children>;

    /// Returns an [`Iterator`] of [`Entity`]s over all of `entity`s ancestors.
    ///
    /// Can only be called on a [`Query`] of [`Parent`] (i.e. `Query<&Parent>`).
    ///
    /// # Examples
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_hierarchy::prelude::*;
    /// # #[derive(Component)]
    /// # struct Marker;
    /// fn system(query: Query<Entity, With<Marker>>, parent_query: Query<&Parent>) {
    ///     let entity = query.single();
    ///     for ancestor in parent_query.iter_ancestors(entity) {
    ///         // Do something!
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    fn iter_ancestors(&'w self, entity: Entity) -> AncestorIter<'w, 's, D, F>
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w Parent>;
}

impl<'w, 's, D: QueryData, F: QueryFilter> HierarchyQueryExt<'w, 's, D, F> for Query<'w, 's, D, F> {
    fn iter_descendants(&'w self, entity: Entity) -> DescendantIter<'w, 's, D, F>
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w Children>,
    {
        DescendantIter::new(self, entity)
    }

    fn iter_ancestors(&'w self, entity: Entity) -> AncestorIter<'w, 's, D, F>
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w Parent>,
    {
        AncestorIter::new(self, entity)
    }
}

/// An [`Iterator`] of [`Entity`]s over the descendants of an [`Entity`].
///
/// Traverses the hierarchy breadth-first.
pub struct DescendantIter<'w, 's, D: QueryData, F: QueryFilter>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w Children>,
{
    children_query: &'w Query<'w, 's, D, F>,
    vecdeque: VecDeque<Entity>,
}

impl<'w, 's, D: QueryData, F: QueryFilter> DescendantIter<'w, 's, D, F>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w Children>,
{
    /// Returns a new [`DescendantIter`].
    pub fn new(children_query: &'w Query<'w, 's, D, F>, entity: Entity) -> Self {
        DescendantIter {
            children_query,
            vecdeque: children_query
                .get(entity)
                .into_iter()
                .flatten()
                .copied()
                .collect(),
        }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter> Iterator for DescendantIter<'w, 's, D, F>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w Children>,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        let entity = self.vecdeque.pop_front()?;

        if let Ok(children) = self.children_query.get(entity) {
            self.vecdeque.extend(children);
        }

        Some(entity)
    }
}

/// An [`Iterator`] of [`Entity`]s over the ancestors of an [`Entity`].
pub struct AncestorIter<'w, 's, D: QueryData, F: QueryFilter>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w Parent>,
{
    parent_query: &'w Query<'w, 's, D, F>,
    next: Option<Entity>,
}

impl<'w, 's, D: QueryData, F: QueryFilter> AncestorIter<'w, 's, D, F>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w Parent>,
{
    /// Returns a new [`AncestorIter`].
    pub fn new(parent_query: &'w Query<'w, 's, D, F>, entity: Entity) -> Self {
        AncestorIter {
            parent_query,
            next: Some(entity),
        }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter> Iterator for AncestorIter<'w, 's, D, F>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w Parent>,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.next = self.parent_query.get(self.next?).ok().map(|p| p.get());
        self.next
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        prelude::Component,
        system::{Query, SystemState},
        world::World,
    };

    use crate::{query_extension::HierarchyQueryExt, BuildWorldChildren, Children, Parent};

    #[derive(Component, PartialEq, Debug)]
    struct A(usize);

    #[test]
    fn descendant_iter() {
        let world = &mut World::new();

        let [a, b, c, d] = std::array::from_fn(|i| world.spawn(A(i)).id());

        world.entity_mut(a).push_children(&[b, c]);
        world.entity_mut(c).push_children(&[d]);

        let mut system_state = SystemState::<(Query<&Children>, Query<&A>)>::new(world);
        let (children_query, a_query) = system_state.get(world);

        let result: Vec<_> = a_query
            .iter_many(children_query.iter_descendants(a))
            .collect();

        assert_eq!([&A(1), &A(2), &A(3)], result.as_slice());
    }

    #[test]
    fn ancestor_iter() {
        let world = &mut World::new();

        let [a, b, c] = std::array::from_fn(|i| world.spawn(A(i)).id());

        world.entity_mut(a).push_children(&[b]);
        world.entity_mut(b).push_children(&[c]);

        let mut system_state = SystemState::<(Query<&Parent>, Query<&A>)>::new(world);
        let (parent_query, a_query) = system_state.get(world);

        let result: Vec<_> = a_query.iter_many(parent_query.iter_ancestors(c)).collect();

        assert_eq!([&A(1), &A(0)], result.as_slice());
    }
}
