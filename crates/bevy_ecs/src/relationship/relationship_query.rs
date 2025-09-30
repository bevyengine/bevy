use crate::{
    entity::Entity,
    query::{QueryData, QueryFilter},
    relationship::{Relationship, RelationshipTarget},
    system::Query,
};
use alloc::collections::VecDeque;
use smallvec::SmallVec;

use super::SourceIter;

impl<'w, 's, D: QueryData, F: QueryFilter> Query<'w, 's, D, F> {
    /// If the given `entity` contains the `R` [`Relationship`] component, returns the
    /// target entity of that relationship.
    pub fn related<R: Relationship>(&'w self, entity: Entity) -> Option<Entity>
    where
        <D as QueryData>::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
    {
        self.get(entity).map(R::get).ok()
    }

    /// If the given `entity` contains the `S` [`RelationshipTarget`] component, returns the
    /// source entities stored on that component.
    pub fn relationship_sources<S: RelationshipTarget>(
        &'w self,
        entity: Entity,
    ) -> impl Iterator<Item = Entity> + 'w
    where
        <D as QueryData>::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
    {
        self.get(entity)
            .into_iter()
            .flat_map(RelationshipTarget::iter)
    }

    /// Recursively walks up the tree defined by the given `R` [`Relationship`] until
    /// there are no more related entities, returning the "root entity" of the relationship hierarchy.
    ///
    /// # Warning
    ///
    /// For relationship graphs that contain loops, this could loop infinitely.
    /// If your relationship is not a tree (like Bevy's hierarchy), be sure to stop if you encounter a duplicate entity.
    pub fn root_ancestor<R: Relationship>(&'w self, entity: Entity) -> Entity
    where
        <D as QueryData>::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
    {
        // Recursively search up the tree until we're out of parents
        match self.get(entity) {
            Ok(parent) => self.root_ancestor(parent.get()),
            Err(_) => entity,
        }
    }

    /// Iterates all "leaf entities" as defined by the [`RelationshipTarget`] hierarchy.
    ///
    /// # Warning
    ///
    /// For relationship graphs that contain loops, this could loop infinitely.
    /// If your relationship is not a tree (like Bevy's hierarchy), be sure to stop if you encounter a duplicate entity.
    pub fn iter_leaves<S: RelationshipTarget>(
        &'w self,
        entity: Entity,
    ) -> impl Iterator<Item = Entity> + use<'w, 's, S, D, F>
    where
        <D as QueryData>::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
        SourceIter<'w, S>: DoubleEndedIterator,
    {
        self.iter_descendants_depth_first(entity).filter(|entity| {
            self.get(*entity)
                // These are leaf nodes if they have the `Children` component but it's empty
                .map(|children| children.len() == 0)
                // Or if they don't have the `Children` component at all
                .unwrap_or(true)
        })
    }

    /// Iterates all sibling entities that also have the `R` [`Relationship`] with the same target entity.
    pub fn iter_siblings<R: Relationship>(
        &'w self,
        entity: Entity,
    ) -> impl Iterator<Item = Entity> + 'w
    where
        D::ReadOnly: QueryData<Item<'w, 's> = (Option<&'w R>, Option<&'w R::RelationshipTarget>)>,
    {
        self.get(entity)
            .ok()
            .and_then(|(maybe_parent, _)| maybe_parent.map(R::get))
            .and_then(|parent| self.get(parent).ok())
            .and_then(|(_, maybe_children)| maybe_children)
            .into_iter()
            .flat_map(move |children| children.iter().filter(move |child| *child != entity))
    }

    /// Iterates all descendant entities as defined by the given `entity`'s [`RelationshipTarget`] and their recursive
    /// [`RelationshipTarget`].
    ///
    /// # Warning
    ///
    /// For relationship graphs that contain loops, this could loop infinitely.
    /// If your relationship is not a tree (like Bevy's hierarchy), be sure to stop if you encounter a duplicate entity.
    pub fn iter_descendants<S: RelationshipTarget>(
        &'w self,
        entity: Entity,
    ) -> DescendantIter<'w, 's, D, F, S>
    where
        D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
    {
        DescendantIter::new(self, entity)
    }

    /// Iterates all descendant entities as defined by the given `entity`'s [`RelationshipTarget`] and their recursive
    /// [`RelationshipTarget`] in depth-first order.
    ///
    /// # Warning
    ///
    /// For relationship graphs that contain loops, this could loop infinitely.
    /// If your relationship is not a tree (like Bevy's hierarchy), be sure to stop if you encounter a duplicate entity.
    pub fn iter_descendants_depth_first<S: RelationshipTarget>(
        &'w self,
        entity: Entity,
    ) -> DescendantDepthFirstIter<'w, 's, D, F, S>
    where
        D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
        SourceIter<'w, S>: DoubleEndedIterator,
    {
        DescendantDepthFirstIter::new(self, entity)
    }

    /// Iterates all ancestors of the given `entity` as defined by the `R` [`Relationship`].
    ///
    /// # Warning
    ///
    /// For relationship graphs that contain loops, this could loop infinitely.
    /// If your relationship is not a tree (like Bevy's hierarchy), be sure to stop if you encounter a duplicate entity.
    pub fn iter_ancestors<R: Relationship>(
        &'w self,
        entity: Entity,
    ) -> AncestorIter<'w, 's, D, F, R>
    where
        D::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
    {
        AncestorIter::new(self, entity)
    }
}

/// An [`Iterator`] of [`Entity`]s over the descendants of an [`Entity`].
///
/// Traverses the hierarchy breadth-first.
pub struct DescendantIter<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipTarget>
where
    D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
{
    children_query: &'w Query<'w, 's, D, F>,
    vecdeque: VecDeque<Entity>,
}

impl<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipTarget> DescendantIter<'w, 's, D, F, S>
where
    D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
{
    /// Returns a new [`DescendantIter`].
    pub fn new(children_query: &'w Query<'w, 's, D, F>, entity: Entity) -> Self {
        DescendantIter {
            children_query,
            vecdeque: children_query
                .get(entity)
                .into_iter()
                .flat_map(RelationshipTarget::iter)
                .collect(),
        }
    }

    /// Creates an iterator which uses a closure to determine if recursive [`RelationshipTarget`]s
    /// should be yielded.
    ///
    /// Once the the provided closure returns `false` for an [`Entity`] it and its recursive
    /// [`RelationshipTarget`]s will not be yielded, effectively skipping that sub hierarchy.
    pub fn filter_hierarchies<HF>(self, filter: HF) -> FilterDescendantIter<'w, 's, D, F, S, HF>
    where
        HF: FnMut(&Entity) -> bool,
    {
        FilterDescendantIter {
            iter: self,
            hierarchy_filter: filter,
        }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipTarget> Iterator
    for DescendantIter<'w, 's, D, F, S>
where
    D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        let entity = self.vecdeque.pop_front()?;

        if let Ok(children) = self.children_query.get(entity) {
            self.vecdeque.extend(children.iter());
        }

        Some(entity)
    }
}

/// An [`Iterator`] of [`Entity`]s over the descendants of an [`Entity`].
///
/// Traverses the hierarchy depth-first.
pub struct DescendantDepthFirstIter<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipTarget>
where
    D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
{
    children_query: &'w Query<'w, 's, D, F>,
    stack: SmallVec<[Entity; 8]>,
}

impl<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipTarget>
    DescendantDepthFirstIter<'w, 's, D, F, S>
where
    D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
    SourceIter<'w, S>: DoubleEndedIterator,
{
    /// Returns a new [`DescendantDepthFirstIter`].
    pub fn new(children_query: &'w Query<'w, 's, D, F>, entity: Entity) -> Self {
        DescendantDepthFirstIter {
            children_query,
            stack: children_query
                .get(entity)
                .map_or(SmallVec::new(), |children| children.iter().rev().collect()),
        }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipTarget> Iterator
    for DescendantDepthFirstIter<'w, 's, D, F, S>
where
    D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
    SourceIter<'w, S>: DoubleEndedIterator,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        let entity = self.stack.pop()?;

        if let Ok(children) = self.children_query.get(entity) {
            self.stack.extend(children.iter().rev());
        }

        Some(entity)
    }
}

/// An [`Iterator`] of [`Entity`]s over the ancestors of an [`Entity`].
pub struct AncestorIter<'w, 's, D: QueryData, F: QueryFilter, R: Relationship>
where
    D::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
{
    parent_query: &'w Query<'w, 's, D, F>,
    next: Option<Entity>,
}

impl<'w, 's, D: QueryData, F: QueryFilter, R: Relationship> AncestorIter<'w, 's, D, F, R>
where
    D::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
{
    /// Returns a new [`AncestorIter`].
    pub fn new(parent_query: &'w Query<'w, 's, D, F>, entity: Entity) -> Self {
        AncestorIter {
            parent_query,
            next: Some(entity),
        }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter, R: Relationship> Iterator
    for AncestorIter<'w, 's, D, F, R>
where
    D::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.next = self.parent_query.get(self.next?).ok().map(R::get);
        self.next
    }
}

/// An [`Iterator`] of [`Entity`]s over the descendants of an [`Entity`].
///
/// Allows conditional skipping of sub hierarchies.
pub struct FilterDescendantIter<'w, 's, D, QF, S, HF>
where
    D: QueryData,
    D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
    QF: QueryFilter,
    S: RelationshipTarget,
    HF: FnMut(&Entity) -> bool,
{
    iter: DescendantIter<'w, 's, D, QF, S>,
    hierarchy_filter: HF,
}

impl<'w, 's, D, QF, S, HF> Iterator for FilterDescendantIter<'w, 's, D, QF, S, HF>
where
    D: QueryData,
    D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
    QF: QueryFilter,
    S: RelationshipTarget,
    HF: FnMut(&Entity) -> bool,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        let mut entity = self.iter.vecdeque.pop_front()?;

        while !(self.hierarchy_filter)(&entity) {
            entity = self.iter.vecdeque.pop_front()?;
        }

        if let Ok(children) = self.iter.children_query.get(entity) {
            self.iter.vecdeque.extend(children.iter());
        }

        Some(entity)
    }
}

#[cfg(test)]
mod test_iter_descendants {
    use crate::{
        prelude::*,
        system::{RunSystemError, RunSystemOnce},
    };
    use alloc::{vec, vec::Vec};

    #[test]
    fn filter_hierarchies_all() -> Result<(), RunSystemError> {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let children = vec![
            world.spawn(ChildOf(root)).id(),
            world.spawn(ChildOf(root)).id(),
            world.spawn(ChildOf(root)).id(),
        ];

        let descendants = world.run_system_once(move |q: Query<&Children>| {
            q.iter_descendants(root)
                .filter_hierarchies(|_| true)
                .collect::<Vec<_>>()
        })?;

        assert_eq!(descendants, children);
        Ok(())
    }

    #[test]
    fn filter_hierarchies_skip_flat() -> Result<(), RunSystemError> {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let c0 = world.spawn(ChildOf(root)).id();
        let c_skip = world.spawn(ChildOf(root)).id();
        let c2 = world.spawn(ChildOf(root)).id();

        let descendants = world.run_system_once(move |q: Query<&Children>| {
            q.iter_descendants(root)
                .filter_hierarchies(|e| e != &c_skip)
                .collect::<Vec<_>>()
        })?;

        assert_eq!(descendants, vec![c0, c2]);
        Ok(())
    }

    #[test]
    fn filter_hierarchies_skip_sub_hierarchy() -> Result<(), RunSystemError> {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let c0 = world.spawn(ChildOf(root)).id();
        let c_skip = world.spawn((ChildOf(root), children![(), ()])).id();
        let c2 = world.spawn(ChildOf(root)).id();

        let descendants = world.run_system_once(move |q: Query<&Children>| {
            q.iter_descendants(root)
                .filter_hierarchies(|e| e != &c_skip)
                .collect::<Vec<_>>()
        })?;

        assert_eq!(descendants, vec![c0, c2]);
        Ok(())
    }
}
