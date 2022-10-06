use std::collections::VecDeque;

use bevy_ecs::{
    entity::Entity,
    query::{ReadOnlyWorldQuery, WorldQuery, WorldQueryGats},
    system::Query,
};

use crate::{Children, Parent};

/// Extension trait for [`Query`]
pub trait HierarchyQueryExt<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery> {
    /// Returns an [`Iterator`] of [`Entity`]s over all of `entity`s descendants.
    ///
    /// Traverses the hierarchy breadth-first.
    fn iter_descendants(&'w self, entity: Entity) -> DescendantIter<'w, 's, Q, F>
    where
        Q::ReadOnly: WorldQueryGats<'w, Item = &'w Children>;

    /// Returns an [`Iterator`] of [`Entity`]s over all of `entity`s ancestors.
    fn iter_ancestors(&'w self, entity: Entity) -> AncestorIter<'w, 's, Q, F>
    where
        Q::ReadOnly: WorldQueryGats<'w, Item = &'w Parent>;
}

impl<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery> HierarchyQueryExt<'w, 's, Q, F>
    for Query<'w, 's, Q, F>
{
    fn iter_descendants(&'w self, entity: Entity) -> DescendantIter<'w, 's, Q, F>
    where
        Q::ReadOnly: WorldQueryGats<'w, Item = &'w Children>,
    {
        DescendantIter::new(self, entity)
    }

    fn iter_ancestors(&'w self, entity: Entity) -> AncestorIter<'w, 's, Q, F>
    where
        Q::ReadOnly: WorldQueryGats<'w, Item = &'w Parent>,
    {
        AncestorIter::new(self, entity)
    }
}

/// An [`Iterator`] of [`Entity`]s over the descendants of an [`Entity`].
///
/// Traverses the hierarchy breadth-first.
pub struct DescendantIter<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery>
where
    Q::ReadOnly: WorldQueryGats<'w, Item = &'w Children>,
{
    children_query: &'w Query<'w, 's, Q, F>,
    vecdeque: VecDeque<Entity>,
}

impl<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery> DescendantIter<'w, 's, Q, F>
where
    Q::ReadOnly: WorldQueryGats<'w, Item = &'w Children>,
{
    /// Returns a new [`DescendantIter`].
    pub fn new(children_query: &'w Query<'w, 's, Q, F>, entity: Entity) -> Self {
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

impl<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery> Iterator for DescendantIter<'w, 's, Q, F>
where
    Q::ReadOnly: WorldQueryGats<'w, Item = &'w Children>,
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
pub struct AncestorIter<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery>
where
    Q::ReadOnly: WorldQueryGats<'w, Item = &'w Parent>,
{
    parent_query: &'w Query<'w, 's, Q, F>,
    next: Option<Entity>,
}

impl<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery> AncestorIter<'w, 's, Q, F>
where
    Q::ReadOnly: WorldQueryGats<'w, Item = &'w Parent>,
{
    /// Returns a new [`AncestorIter`].
    pub fn new(parent_query: &'w Query<'w, 's, Q, F>, entity: Entity) -> Self {
        AncestorIter {
            parent_query,
            next: parent_query.get(entity).ok().map(|p| p.get()),
        }
    }
}

impl<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery> Iterator for AncestorIter<'w, 's, Q, F>
where
    Q::ReadOnly: WorldQueryGats<'w, Item = &'w Parent>,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.next?;

        self.next = self.parent_query.get(next).ok().map(|p| p.get());

        Some(next)
    }
}
