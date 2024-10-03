use core::marker::PhantomData;

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
    fn children(&'w self, entity: Entity) -> impl Iterator<Item = Entity> + 'w
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w Children>;

    /// Returns the topmost ancestor of the given `entity`.
    ///
    /// This may be the entity itself if it has no parent.
    fn root_parent(&'w self, entity: Entity) -> Entity
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w Parent>;

    /// Returns an [`Iterator`] of [`Entity`]s over the leaves of the hierarchy that are underneath this `entity`.
    ///
    /// Only entities which have no children are considered leaves.
    /// This will not include the entity itself, and will not include any entities which are not descendants of the entity,
    /// even if they are leaves in the same hierarchical tree.
    fn iter_leaves(&'w self, entity: Entity) -> LeafIter<'w, 's, D, F>
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w Children>;

    /// Returns an [`Iterator`] of [`Entity`]s over the `entity`s immediate siblings, who share them same parent.
    ///
    /// The entity itself is not included in the iterator.
    fn iter_siblings(&'w self, entity: Entity) -> SiblingIter<'w, 's, D, F>
    where
        D::ReadOnly: WorldQuery<Item<'w> = (&'w Parent, &'w Children)>;

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
    /// Does not include the entity itself.
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
    fn parent(&'w self, entity: Entity) -> Option<Entity>
    where
        <D as QueryData>::ReadOnly: WorldQuery<Item<'w> = &'w Parent>,
    {
        self.get(entity).map(Parent::get).ok()
    }

    fn children(&'w self, entity: Entity) -> impl Iterator<Item = Entity>
    where
        <D as QueryData>::ReadOnly: WorldQuery<Item<'w> = &'w Children>,
    {
        // We must return the same type from both branches of the match
        // So we've defined a throwaway enum to wrap the two types
        enum MaybeChildrenIter {
            Children { cursor: usize, vec: Vec<Entity> },
            None,
        }

        impl Iterator for MaybeChildrenIter {
            type Item = Entity;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    MaybeChildrenIter::Children { cursor, vec } => {
                        if *cursor < vec.len() {
                            let entity = vec[*cursor];
                            *cursor += 1;
                            Some(entity)
                        } else {
                            None
                        }
                    }
                    MaybeChildrenIter::None => None,
                }
            }
        }

        match self.get(entity) {
            Ok(children) => MaybeChildrenIter::Children {
                cursor: 0,
                vec: children.to_vec(),
            },
            Err(_) => MaybeChildrenIter::None,
        }
    }

    fn root_parent(&'w self, entity: Entity) -> Entity
    where
        <D as QueryData>::ReadOnly: WorldQuery<Item<'w> = &'w Parent>,
    {
        // Recursively search up the tree until we're out of parents
        match self.get(entity) {
            Ok(parent) => self.root_parent(parent.get()),
            Err(_) => entity,
        }
    }

    fn iter_leaves(&'w self, entity: Entity) -> LeafIter<'w, 's, D, F>
    where
        <D as QueryData>::ReadOnly: WorldQuery<Item<'w> = &'w Children>,
    {
        LeafIter::new(self, entity)
    }

    fn iter_siblings(&'w self, entity: Entity) -> SiblingIter<'w, 's, D, F>
    where
        D::ReadOnly: WorldQuery<Item<'w> = (&'w Parent, &'w Children)>,
    {
        SiblingIter::<D, F>::new(self, entity)
    }

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

/// An [`Iterator`] of [`Entity`]s over the leaf descendants of an [`Entity`].
pub struct LeafIter<'w, 's, D: QueryData, F: QueryFilter>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w Children>,
{
    vecdeque: VecDeque<Entity>,
    // PERF: if this ends up resulting in too much memory being allocated, we can store the query instead
    // like in IterDescendants
    _phantom: PhantomData<(&'w D, &'s F)>,
}

impl<'w, 's, D: QueryData, F: QueryFilter> LeafIter<'w, 's, D, F>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w Children>,
{
    /// Returns a new [`LeafIter`].
    pub fn new(children_query: &'w Query<'w, 's, D, F>, entity: Entity) -> Self {
        let leaf_children = children_query.iter_descendants(entity).filter(|entity| {
            children_query
                .get(*entity)
                // These are leaf nodes if they have the `Children` component but it's empty
                .map(|children| children.is_empty())
                // Or if they don't have the `Children` component at all
                .unwrap_or(true)
        });

        LeafIter {
            vecdeque: leaf_children.collect(),
            _phantom: PhantomData,
        }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter> Iterator for LeafIter<'w, 's, D, F>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w Children>,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        let entity: Entity = self.vecdeque.pop_front()?;
        Some(entity)
    }
}

/// An [`Iterator`] of [`Entity`]s over the siblings of an [`Entity`].
pub struct SiblingIter<'w, 's, D: QueryData, F: QueryFilter>
where
    D::ReadOnly: WorldQuery<Item<'w> = (&'w Parent, &'w Children)>,
{
    // Unlike other iterators, we don't need to store the query here,
    // as the number of siblings is likely to be much smaller than the number of descendants.
    small_vec: SmallVec<[Entity; 8]>,
    _phantom: PhantomData<(&'w D, &'s F)>,
}

impl<'w, 's, D: QueryData, F: QueryFilter> SiblingIter<'w, 's, D, F>
where
    D::ReadOnly: WorldQuery<Item<'w> = (&'w Parent, &'w Children)>,
{
    /// Returns a new [`SiblingIter`].
    pub fn new(hierarchy_query: &'w Query<'w, 's, D, F>, entity: Entity) -> Self {
        match hierarchy_query.get(entity) {
            Ok((parent, _)) => {
                let Ok((_, children_of_parent)) = hierarchy_query.get(parent.get()) else {
                    return SiblingIter {
                        small_vec: SmallVec::new(),
                        _phantom: PhantomData,
                    };
                };

                let siblings = children_of_parent.iter().filter(|child| **child != entity);

                SiblingIter {
                    small_vec: SmallVec::from_iter(siblings.copied()),
                    _phantom: PhantomData,
                }
            }
            Err(_) => SiblingIter {
                small_vec: SmallVec::new(),
                _phantom: PhantomData,
            },
        }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter> Iterator for SiblingIter<'w, 's, D, F>
where
    D::ReadOnly: WorldQuery<Item<'w> = (&'w Parent, &'w Children)>,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        let entity: Entity = self.small_vec.pop()?;
        Some(entity)
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
    fn root_parent() {
        let world = &mut World::new();

        let [a, b, c] = core::array::from_fn(|i| world.spawn(A(i)).id());

        world.entity_mut(a).add_children(&[b]);
        world.entity_mut(b).add_children(&[c]);

        let mut system_state = SystemState::<Query<&Parent>>::new(world);
        let parent_query = system_state.get(world);

        assert_eq!(a, parent_query.root_parent(c));
        assert_eq!(a, parent_query.root_parent(b));
        assert_eq!(a, parent_query.root_parent(a));
    }

    #[test]
    fn leaf_iter() {
        let world = &mut World::new();

        let [a, b, c, d] = core::array::from_fn(|i| world.spawn(A(i)).id());

        world.entity_mut(a).add_children(&[b, c]);
        world.entity_mut(c).add_children(&[d]);

        let mut system_state = SystemState::<(Query<&Children>, Query<&A>)>::new(world);
        let (children_query, a_query) = system_state.get(world);

        let result: Vec<_> = a_query.iter_many(children_query.iter_leaves(a)).collect();

        assert_eq!([&A(1), &A(3)], result.as_slice());
    }

    #[test]
    fn siblings() {
        let world = &mut World::new();

        let [a, b, c, d, e] = core::array::from_fn(|i| world.spawn(A(i)).id());

        world.entity_mut(a).add_children(&[b, c, d]);
        world.entity_mut(c).add_children(&[e]);

        let mut system_state = SystemState::<(Query<(&Parent, &Children)>, Query<&A>)>::new(world);
        let (hierarchy_query, a_query) = system_state.get(world);

        let result: Vec<_> = a_query
            .iter_many(hierarchy_query.iter_siblings(b))
            .collect();

        assert_eq!([&A(2), &A(3)], result.as_slice());
    }
}
