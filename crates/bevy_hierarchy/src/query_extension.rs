use alloc::collections::VecDeque;

use bevy_ecs::{
    entity::Entity,
    query::{QueryData, QueryFilter, WorldQuery},
    system::Query,
};
use smallvec::SmallVec;

use crate::{Children, Parent};

/// An extension trait for [`Query`] that adds hierarchy related methods.
pub trait HierarchyQueryExt<'w, 's, D: QueryData, F: QueryFilter> {
    /// Returns the parent [`Entity`] of the given `entity`, if any.
    fn parent(&'w self, entity: Entity) -> Option<Entity>
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w Parent>;

    /// Returns a slice over the [`Children`] of the given `entity`.
    ///
    /// This may be empty if the `entity` has no children.
    fn children(&'w self, entity: Entity) -> &'w [Entity]
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w Children>;

    /// Returns the topmost ancestor of the given `entity`.
    ///
    /// This may be the entity itself if it has no parent.
    fn root_ancestor(&'w self, entity: Entity) -> Entity
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w Parent>;

    /// Returns an [`Iterator`] of [`Entity`]s over the leaves of the hierarchy that are underneath this `entity`.
    ///
    /// Only entities which have no children are considered leaves.
    /// This will not include the entity itself, and will not include any entities which are not descendants of the entity,
    /// even if they are leaves in the same hierarchical tree.
    ///
    /// Traverses the hierarchy depth-first.
    fn iter_leaves(&'w self, entity: Entity) -> impl Iterator<Item = Entity> + 'w
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w Children>;

    /// Returns an [`Iterator`] of [`Entity`]s over the `entity`s immediate siblings, who share the same parent.
    ///
    /// The entity itself is not included in the iterator.
    fn iter_siblings(&'w self, entity: Entity) -> impl Iterator<Item = Entity>
    where
        D::ReadOnly: WorldQuery<Item<'w> = (Option<&'w Parent>, Option<&'w Children>)>;

    /// Returns an [`Iterator`] of [`Entity`]s over all of `entity`s descendants.
    ///
    /// Can only be called on a [`Query`] of [`Children`] (i.e. `Query<&Children>`).
    ///
    /// Traverses the hierarchy breadth-first and does not include the entity itself.
    ///
    /// # Examples
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_hierarchy::prelude::*;
    /// # #[derive(Component)]
    /// # struct Marker;
    /// fn system(entity: Single<Entity, With<Marker>>, children_query: Query<&Children>) {
    ///     for descendant in children_query.iter_descendants(*entity) {
    ///         // Do something!
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    fn iter_descendants(&'w self, entity: Entity) -> DescendantIter<'w, 's, D, F>
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w Children>;

    /// Returns an [`Iterator`] of [`Entity`]s over all of `entity`s descendants.
    ///
    /// Can only be called on a [`Query`] of [`Children`] (i.e. `Query<&Children>`).
    ///
    /// This is a depth-first alternative to [`HierarchyQueryExt::iter_descendants`].
    fn iter_descendants_depth_first(
        &'w self,
        entity: Entity,
    ) -> DescendantDepthFirstIter<'w, 's, D, F>
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w Children>;

    /// Returns an [`Iterator`] of [`Entity`]s over all of `entity`s ancestors.
    ///
    /// Does not include the entity itself.
    /// Can only be called on a [`Query`] of [`Parent`] (i.e. `Query<&Parent>`).
    ///
    /// # Examples
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_hierarchy::prelude::*;
    /// # #[derive(Component)]
    /// # struct Marker;
    /// fn system(entity: Single<Entity, With<Marker>>, parent_query: Query<&Parent>) {
    ///     for ancestor in parent_query.iter_ancestors(*entity) {
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
    fn parent(&'w self, entity: Entity) -> Option<Entity>
    where
        <D as QueryData>::ReadOnly: WorldQuery<Item<'w> = &'w Parent>,
    {
        self.get(entity).map(Parent::get).ok()
    }

    fn children(&'w self, entity: Entity) -> &'w [Entity]
    where
        <D as QueryData>::ReadOnly: WorldQuery<Item<'w> = &'w Children>,
    {
        self.get(entity)
            .map_or(&[] as &[Entity], |children| children)
    }

    fn root_ancestor(&'w self, entity: Entity) -> Entity
    where
        <D as QueryData>::ReadOnly: WorldQuery<Item<'w> = &'w Parent>,
    {
        // Recursively search up the tree until we're out of parents
        match self.get(entity) {
            Ok(parent) => self.root_ancestor(parent.get()),
            Err(_) => entity,
        }
    }

    fn iter_leaves(&'w self, entity: Entity) -> impl Iterator<Item = Entity>
    where
        <D as QueryData>::ReadOnly: WorldQuery<Item<'w> = &'w Children>,
    {
        self.iter_descendants_depth_first(entity).filter(|entity| {
            self.get(*entity)
                // These are leaf nodes if they have the `Children` component but it's empty
                .map(|children| children.is_empty())
                // Or if they don't have the `Children` component at all
                .unwrap_or(true)
        })
    }

    fn iter_siblings(&'w self, entity: Entity) -> impl Iterator<Item = Entity>
    where
        D::ReadOnly: WorldQuery<Item<'w> = (Option<&'w Parent>, Option<&'w Children>)>,
    {
        self.get(entity)
            .ok()
            .and_then(|(maybe_parent, _)| maybe_parent.map(Parent::get))
            .and_then(|parent| self.get(parent).ok())
            .and_then(|(_, maybe_children)| maybe_children)
            .into_iter()
            .flat_map(move |children| children.iter().filter(move |child| **child != entity))
            .copied()
    }

    fn iter_descendants(&'w self, entity: Entity) -> DescendantIter<'w, 's, D, F>
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w Children>,
    {
        DescendantIter::new(self, entity)
    }

    fn iter_descendants_depth_first(
        &'w self,
        entity: Entity,
    ) -> DescendantDepthFirstIter<'w, 's, D, F>
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w Children>,
    {
        DescendantDepthFirstIter::new(self, entity)
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

/// An [`Iterator`] of [`Entity`]s over the descendants of an [`Entity`].
///
/// Traverses the hierarchy depth-first.
pub struct DescendantDepthFirstIter<'w, 's, D: QueryData, F: QueryFilter>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w Children>,
{
    children_query: &'w Query<'w, 's, D, F>,
    stack: SmallVec<[Entity; 8]>,
}

impl<'w, 's, D: QueryData, F: QueryFilter> DescendantDepthFirstIter<'w, 's, D, F>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w Children>,
{
    /// Returns a new [`DescendantDepthFirstIter`].
    pub fn new(children_query: &'w Query<'w, 's, D, F>, entity: Entity) -> Self {
        DescendantDepthFirstIter {
            children_query,
            stack: children_query
                .get(entity)
                .map_or(SmallVec::new(), |children| {
                    children.iter().rev().copied().collect()
                }),
        }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter> Iterator for DescendantDepthFirstIter<'w, 's, D, F>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w Children>,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        let entity = self.stack.pop()?;

        if let Ok(children) = self.children_query.get(entity) {
            self.stack.extend(children.iter().rev().copied());
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
        self.next = self.parent_query.get(self.next?).ok().map(Parent::get);
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

    use crate::{query_extension::HierarchyQueryExt, BuildChildren, Children, Parent};

    #[derive(Component, PartialEq, Debug)]
    struct A(usize);

    #[test]
    fn descendant_iter() {
        let world = &mut World::new();

        let [a0, a1, a2, a3] = core::array::from_fn(|i| world.spawn(A(i)).id());

        world.entity_mut(a0).add_children(&[a1, a2]);
        world.entity_mut(a1).add_children(&[a3]);

        let mut system_state = SystemState::<(Query<&Children>, Query<&A>)>::new(world);
        let (children_query, a_query) = system_state.get(world);

        let result: Vec<_> = a_query
            .iter_many(children_query.iter_descendants(a0))
            .collect();

        assert_eq!([&A(1), &A(2), &A(3)], result.as_slice());
    }

    #[test]
    fn descendant_depth_first_iter() {
        let world = &mut World::new();

        let [a0, a1, a2, a3] = core::array::from_fn(|i| world.spawn(A(i)).id());

        world.entity_mut(a0).add_children(&[a1, a2]);
        world.entity_mut(a1).add_children(&[a3]);

        let mut system_state = SystemState::<(Query<&Children>, Query<&A>)>::new(world);
        let (children_query, a_query) = system_state.get(world);

        let result: Vec<_> = a_query
            .iter_many(children_query.iter_descendants_depth_first(a0))
            .collect();

        assert_eq!([&A(1), &A(3), &A(2)], result.as_slice());
    }

    #[test]
    fn ancestor_iter() {
        let world = &mut World::new();

        let [a0, a1, a2] = core::array::from_fn(|i| world.spawn(A(i)).id());

        world.entity_mut(a0).add_children(&[a1]);
        world.entity_mut(a1).add_children(&[a2]);

        let mut system_state = SystemState::<(Query<&Parent>, Query<&A>)>::new(world);
        let (parent_query, a_query) = system_state.get(world);

        let result: Vec<_> = a_query.iter_many(parent_query.iter_ancestors(a2)).collect();

        assert_eq!([&A(1), &A(0)], result.as_slice());
    }

    #[test]
    fn root_ancestor() {
        let world = &mut World::new();

        let [a0, a1, a2] = core::array::from_fn(|i| world.spawn(A(i)).id());

        world.entity_mut(a0).add_children(&[a1]);
        world.entity_mut(a1).add_children(&[a2]);

        let mut system_state = SystemState::<Query<&Parent>>::new(world);
        let parent_query = system_state.get(world);

        assert_eq!(a0, parent_query.root_ancestor(a2));
        assert_eq!(a0, parent_query.root_ancestor(a1));
        assert_eq!(a0, parent_query.root_ancestor(a0));
    }

    #[test]
    fn leaf_iter() {
        let world = &mut World::new();

        let [a0, a1, a2, a3] = core::array::from_fn(|i| world.spawn(A(i)).id());

        world.entity_mut(a0).add_children(&[a1, a2]);
        world.entity_mut(a1).add_children(&[a3]);

        let mut system_state = SystemState::<(Query<&Children>, Query<&A>)>::new(world);
        let (children_query, a_query) = system_state.get(world);

        let result: Vec<_> = a_query.iter_many(children_query.iter_leaves(a0)).collect();

        assert_eq!([&A(3), &A(2)], result.as_slice());
    }

    #[test]
    fn siblings() {
        let world = &mut World::new();

        let [a0, a1, a2, a3, a4] = core::array::from_fn(|i| world.spawn(A(i)).id());

        world.entity_mut(a0).add_children(&[a1, a2, a3]);
        world.entity_mut(a2).add_children(&[a4]);

        let mut system_state =
            SystemState::<(Query<(Option<&Parent>, Option<&Children>)>, Query<&A>)>::new(world);
        let (hierarchy_query, a_query) = system_state.get(world);

        let result: Vec<_> = a_query
            .iter_many(hierarchy_query.iter_siblings(a1))
            .collect();

        assert_eq!([&A(2), &A(3)], result.as_slice());
    }
}
