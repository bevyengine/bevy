use crate::{
    entity::Entity,
    query::{QueryData, QueryStateDeref},
    relationship::{Relationship, RelationshipTarget},
    system::Query,
};
use alloc::collections::VecDeque;
use smallvec::SmallVec;

use super::SourceIter;

impl<'w, 's, D: QueryData, S: QueryStateDeref<Data = D>> Query<'w, 's, D, S::Filter, S> {
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
    pub fn relationship_sources<R: RelationshipTarget>(
        &'w self,
        entity: Entity,
    ) -> impl Iterator<Item = Entity> + 'w
    where
        <D as QueryData>::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
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
    pub fn iter_leaves<R: RelationshipTarget>(
        &'w self,
        entity: Entity,
    ) -> impl Iterator<Item = Entity> + use<'w, 's, S, D, F>
    where
        <D as QueryData>::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
        SourceIter<'w, R>: DoubleEndedIterator,
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
    pub fn iter_descendants<R: RelationshipTarget>(
        &'w self,
        entity: Entity,
    ) -> DescendantIter<'w, 's, S, R>
    where
        D::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
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
    pub fn iter_descendants_depth_first<R: RelationshipTarget>(
        &'w self,
        entity: Entity,
    ) -> DescendantDepthFirstIter<'w, 's, S, R>
    where
        D::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
        SourceIter<'w, R>: DoubleEndedIterator,
    {
        DescendantDepthFirstIter::new(self, entity)
    }

    /// Iterates all ancestors of the given `entity` as defined by the `R` [`Relationship`].
    ///
    /// # Warning
    ///
    /// For relationship graphs that contain loops, this could loop infinitely.
    /// If your relationship is not a tree (like Bevy's hierarchy), be sure to stop if you encounter a duplicate entity.
    pub fn iter_ancestors<R: Relationship>(&'w self, entity: Entity) -> AncestorIter<'w, 's, S, R>
    where
        D::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
    {
        AncestorIter::new(self, entity)
    }
}

/// An [`Iterator`] of [`Entity`]s over the descendants of an [`Entity`].
///
/// Traverses the hierarchy breadth-first.
pub struct DescendantIter<'w, 's, S: QueryStateDeref, R: RelationshipTarget>
where
    <S::Data as QueryData>::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
{
    children_query: &'w Query<'w, 's, S::Data, S::Filter, S>,
    vecdeque: VecDeque<Entity>,
}

impl<'w, 's, S: QueryStateDeref, R: RelationshipTarget> DescendantIter<'w, 's, S, R>
where
    <S::Data as QueryData>::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
{
    /// Returns a new [`DescendantIter`].
    pub fn new(children_query: &'w Query<'w, 's, S::Data, S::Filter, S>, entity: Entity) -> Self {
        DescendantIter {
            children_query,
            vecdeque: children_query
                .get(entity)
                .into_iter()
                .flat_map(RelationshipTarget::iter)
                .collect(),
        }
    }
}

impl<'w, 's, S: QueryStateDeref, R: RelationshipTarget> Iterator for DescendantIter<'w, 's, S, R>
where
    <S::Data as QueryData>::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
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
pub struct DescendantDepthFirstIter<'w, 's, S: QueryStateDeref, R: RelationshipTarget>
where
    <S::Data as QueryData>::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
{
    children_query: &'w Query<'w, 's, S::Data, S::Filter, S>,
    stack: SmallVec<[Entity; 8]>,
}

impl<'w, 's, S: QueryStateDeref, R: RelationshipTarget> DescendantDepthFirstIter<'w, 's, S, R>
where
    <S::Data as QueryData>::ReadOnly: QueryData<Item<'w, s> = &'w R>,
    SourceIter<'w, R>: DoubleEndedIterator,
{
    /// Returns a new [`DescendantDepthFirstIter`].
    pub fn new(children_query: &'w Query<'w, 's, S::Data, S::Filter, S>, entity: Entity) -> Self {
        DescendantDepthFirstIter {
            children_query,
            stack: children_query
                .get(entity)
                .map_or(SmallVec::new(), |children| children.iter().rev().collect()),
        }
    }
}

impl<'w, 's, S: QueryStateDeref, R: RelationshipTarget> Iterator
    for DescendantDepthFirstIter<'w, 's, S, R>
where
    <S::Data as QueryData>::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
    SourceIter<'w, R>: DoubleEndedIterator,
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
pub struct AncestorIter<'w, 's, S: QueryStateDeref, R: Relationship>
where
    <S::Data as QueryData>::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
{
    parent_query: &'w Query<'w, 's, S::Data, S::Filter, S>,
    next: Option<Entity>,
}

impl<'w, 's, S: QueryStateDeref, R: Relationship> AncestorIter<'w, 's, S, R>
where
    <S::Data as QueryData>::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
{
    /// Returns a new [`AncestorIter`].
    pub fn new(parent_query: &'w Query<'w, 's, S::Data, S::Filter, S>, entity: Entity) -> Self {
        AncestorIter {
            parent_query,
            next: Some(entity),
        }
    }
}

impl<'w, 's, S: QueryStateDeref, R: Relationship> Iterator for AncestorIter<'w, 's, S, R>
where
    <S::Data as QueryData>::ReadOnly: QueryData<Item<'w, 's> = &'w R>,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.next = self.parent_query.get(self.next?).ok().map(R::get);
        self.next
    }
}
