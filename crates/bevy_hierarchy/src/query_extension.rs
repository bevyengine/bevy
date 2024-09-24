use alloc::collections::VecDeque;

use bevy_ecs::{
    component::Component,
    entity::{Entity, VisitEntities},
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
    fn iter_descendants(&'w self, entity: Entity) -> RelatedIter<'w, 's, Children, D, F>
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w Children>,
    {
        self.iter_related(entity)
    }

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
    fn iter_ancestors(&'w self, entity: Entity) -> RelatedIter<'w, 's, Parent, D, F>
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w Parent>,
    {
        self.iter_related(entity)
    }

    /// Returns an [`Iterator`] of [`Entity`]'s over all of `entity`'s `C` relations.
    ///
    /// Can only be called on a [`Query`] of `C` (i.e. `Query<&C>`), where `C`
    /// is some type implementing [`VisitEntities`].
    /// # Examples
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_hierarchy::prelude::*;
    /// # #[derive(Component)]
    /// # struct Marker;
    /// fn system(query: Query<Entity, With<Marker>>, parent_query: Query<&Parent>) {
    ///     let entity = query.single();
    ///     for ancestor in parent_query.iter_related(entity) {
    ///         // Do something!
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    fn iter_related<C>(&'w self, entity: Entity) -> RelatedIter<'w, 's, C, D, F>
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w C>,
        C: Component + VisitEntities;
}

impl<'w, 's, D: QueryData, F: QueryFilter> HierarchyQueryExt<'w, 's, D, F> for Query<'w, 's, D, F> {
    fn iter_related<C>(&'w self, entity: Entity) -> RelatedIter<'w, 's, C, D, F>
    where
        <D as QueryData>::ReadOnly: WorldQuery<Item<'w> = &'w C>,
        C: Component + VisitEntities,
    {
        RelatedIter::new(self, entity)
    }
}

/// An iterator over entities related via some component `C` that implements
/// [`VisitEntities`].
///
/// Traverses the hierarchy breadth-first.
pub struct RelatedIter<'w, 's, C, D, F>
where
    D: QueryData,
    F: QueryFilter,
    D::ReadOnly: WorldQuery<Item<'w> = &'w C>,
    C: Component + VisitEntities,
{
    related_query: &'w Query<'w, 's, D, F>,
    vecdeque: VecDeque<(usize, Entity)>,
}

impl<'w, 's, C, D, F> RelatedIter<'w, 's, C, D, F>
where
    D: QueryData,
    F: QueryFilter,
    D::ReadOnly: WorldQuery<Item<'w> = &'w C>,
    C: Component + VisitEntities,
{
    /// Create a new [`RelatedIter`].
    fn new(related_query: &'w Query<'w, 's, D, F>, entity: Entity) -> Self {
        let mut vecdeque = VecDeque::new();
        related_query.get(entity).into_iter().for_each(|t| {
            t.visit_entities(|entity| {
                vecdeque.push_back((1, entity));
            });
        });
        RelatedIter {
            related_query,
            vecdeque,
        }
    }

    fn next(&mut self) -> Option<(usize, Entity)> {
        let (depth, entity) = self.vecdeque.pop_front()?;

        if let Ok(children) = self.related_query.get(entity) {
            children.visit_entities(|entity| {
                self.vecdeque.push_back((depth + 1, entity));
            });
        }

        Some((depth, entity))
    }

    /// Include depth information with the entities produced by this iterator.
    pub fn with_depth(self) -> RelatedDepthIter<'w, 's, C, D, F> {
        RelatedDepthIter(self)
    }
}

impl<'w, 's, C, D, F> Iterator for RelatedIter<'w, 's, C, D, F>
where
    D: QueryData,
    F: QueryFilter,
    D::ReadOnly: WorldQuery<Item<'w> = &'w C>,
    C: Component + VisitEntities,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        RelatedIter::next(self).map(|(_, e)| e)
    }
}

/// A [`RelatedIter`] that includes depth information.
pub struct RelatedDepthIter<'w, 's, C, D, F>(RelatedIter<'w, 's, C, D, F>)
where
    D: QueryData,
    F: QueryFilter,
    D::ReadOnly: WorldQuery<Item<'w> = &'w C>,
    C: Component + VisitEntities;

impl<'w, 's, C, D, F> Iterator for RelatedDepthIter<'w, 's, C, D, F>
where
    D: QueryData,
    F: QueryFilter,
    D::ReadOnly: WorldQuery<Item<'w> = &'w C>,
    C: Component + VisitEntities,
{
    type Item = (usize, Entity);

    fn next(&mut self) -> Option<Self::Item> {
        RelatedIter::next(&mut self.0)
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        prelude::Component,
        system::{Query, SystemState},
        world::World,
    };

    use crate::{query_extension::HierarchyQueryExt, BuildChildren, Children, Parent};

    #[derive(Component, PartialEq, Debug)]
    struct A(usize);

    #[test]
    fn descendant_iter() {
        let world = &mut World::new();

        let [a, b, c, d] = core::array::from_fn(|i| world.spawn(A(i)).id());

        world.entity_mut(a).add_children(&[b, c]);
        world.entity_mut(c).add_children(&[d]);

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

        let [a, b, c] = core::array::from_fn(|i| world.spawn(A(i)).id());

        world.entity_mut(a).add_children(&[b]);
        world.entity_mut(b).add_children(&[c]);

        let mut system_state = SystemState::<(Query<&Parent>, Query<&A>)>::new(world);
        let (parent_query, a_query) = system_state.get(world);

        let result: Vec<_> = a_query.iter_many(parent_query.iter_ancestors(c)).collect();

        assert_eq!([&A(1), &A(0)], result.as_slice());
    }

    #[test]
    fn related_children_iter() {
        let world = &mut World::new();

        let [a, b, c, d] = core::array::from_fn(|i| world.spawn(A(i)).id());

        world.entity_mut(a).add_children(&[b, c]);
        world.entity_mut(c).add_children(&[d]);

        let mut system_state = SystemState::<(Query<&Children>, Query<&A>)>::new(world);
        let (children_query, a_query) = system_state.get(world);

        let result: Vec<_> = children_query
            .iter_related(a)
            .with_depth()
            .filter_map(|(d, e)| Some((d, a_query.get(e).ok()?)))
            .collect();

        assert_eq!([(1, &A(1)), (1, &A(2)), (2, &A(3))], result.as_slice());
    }

    #[test]
    fn related_parent_iter() {
        let world = &mut World::new();

        let [a, b, c] = core::array::from_fn(|i| world.spawn(A(i)).id());

        world.entity_mut(a).add_children(&[b]);
        world.entity_mut(b).add_children(&[c]);

        let mut system_state = SystemState::<(Query<&Parent>, Query<&A>)>::new(world);
        let (parent_query, a_query) = system_state.get(world);

        let result: Vec<_> = parent_query
            .iter_related(c)
            .with_depth()
            .filter_map(|(d, e)| Some((d, a_query.get(e).ok()?)))
            .collect();

        assert_eq!([(1, &A(1)), (2, &A(0))], result.as_slice());
    }
}
