use crate::{
    entity::Entity,
    query::{QueryData, QueryFilter, WorldQuery},
    relationship::{Relationship, RelationshipSources},
    system::Query,
};
use smallvec::SmallVec;
use std::collections::VecDeque;

impl<'w, 's, D: QueryData, F: QueryFilter> Query<'w, 's, D, F> {
    pub fn related<R: Relationship>(&'w self, entity: Entity) -> Option<Entity>
    where
        <D as QueryData>::ReadOnly: WorldQuery<Item<'w> = &'w R>,
    {
        self.get(entity).map(R::get).ok()
    }

    pub fn relationship_sources<S: RelationshipSources>(
        &'w self,
        entity: Entity,
    ) -> impl Iterator<Item = Entity> + 'w
    where
        <D as QueryData>::ReadOnly: WorldQuery<Item<'w> = &'w S>,
    {
        self.get(entity)
            .into_iter()
            .flat_map(RelationshipSources::iter)
    }

    pub fn root_ancestor<R: Relationship>(&'w self, entity: Entity) -> Entity
    where
        <D as QueryData>::ReadOnly: WorldQuery<Item<'w> = &'w R>,
    {
        // Recursively search up the tree until we're out of parents
        match self.get(entity) {
            Ok(parent) => self.root_ancestor(parent.get()),
            Err(_) => entity,
        }
    }

    pub fn iter_leaves<S: RelationshipSources>(
        &'w self,
        entity: Entity,
    ) -> impl Iterator<Item = Entity> + 'w
    where
        <D as QueryData>::ReadOnly: WorldQuery<Item<'w> = &'w S>,
    {
        self.iter_descendants_depth_first(entity).filter(|entity| {
            self.get(*entity)
                // These are leaf nodes if they have the `Children` component but it's empty
                .map(|children| children.len() == 0)
                // Or if they don't have the `Children` component at all
                .unwrap_or(true)
        })
    }

    pub fn iter_siblings<R: Relationship>(
        &'w self,
        entity: Entity,
    ) -> impl Iterator<Item = Entity> + 'w
    where
        D::ReadOnly: WorldQuery<Item<'w> = (Option<&'w R>, Option<&'w R::RelationshipSources>)>,
    {
        self.get(entity)
            .ok()
            .and_then(|(maybe_parent, _)| maybe_parent.map(R::get))
            .and_then(|parent| self.get(parent).ok())
            .and_then(|(_, maybe_children)| maybe_children)
            .into_iter()
            .flat_map(move |children| children.iter().filter(move |child| *child != entity))
    }

    pub fn iter_descendants<S: RelationshipSources>(
        &'w self,
        entity: Entity,
    ) -> DescendantIter<'w, 's, D, F, S>
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w S>,
    {
        DescendantIter::new(self, entity)
    }

    pub fn iter_descendants_depth_first<S: RelationshipSources>(
        &'w self,
        entity: Entity,
    ) -> DescendantDepthFirstIter<'w, 's, D, F, S>
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w S>,
    {
        DescendantDepthFirstIter::new(self, entity)
    }

    pub fn iter_ancestors<R: Relationship>(
        &'w self,
        entity: Entity,
    ) -> AncestorIter<'w, 's, D, F, R>
    where
        D::ReadOnly: WorldQuery<Item<'w> = &'w R>,
    {
        AncestorIter::new(self, entity)
    }
}

/// An [`Iterator`] of [`Entity`]s over the descendants of an [`Entity`].
///
/// Traverses the hierarchy breadth-first.
pub struct DescendantIter<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipSources>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w S>,
{
    children_query: &'w Query<'w, 's, D, F>,
    vecdeque: VecDeque<Entity>,
}

impl<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipSources> DescendantIter<'w, 's, D, F, S>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w S>,
{
    /// Returns a new [`DescendantIter`].
    pub fn new(children_query: &'w Query<'w, 's, D, F>, entity: Entity) -> Self {
        DescendantIter {
            children_query,
            vecdeque: children_query
                .get(entity)
                .into_iter()
                .flat_map(RelationshipSources::iter)
                .collect(),
        }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipSources> Iterator
    for DescendantIter<'w, 's, D, F, S>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w S>,
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
pub struct DescendantDepthFirstIter<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipSources>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w S>,
{
    children_query: &'w Query<'w, 's, D, F>,
    stack: SmallVec<[Entity; 8]>,
}

impl<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipSources>
    DescendantDepthFirstIter<'w, 's, D, F, S>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w S>,
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

impl<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipSources> Iterator
    for DescendantDepthFirstIter<'w, 's, D, F, S>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w S>,
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
    D::ReadOnly: WorldQuery<Item<'w> = &'w R>,
{
    parent_query: &'w Query<'w, 's, D, F>,
    next: Option<Entity>,
}

impl<'w, 's, D: QueryData, F: QueryFilter, R: Relationship> AncestorIter<'w, 's, D, F, R>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w R>,
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
    D::ReadOnly: WorldQuery<Item<'w> = &'w R>,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.next = self.parent_query.get(self.next?).ok().map(R::get);
        self.next
    }
}
