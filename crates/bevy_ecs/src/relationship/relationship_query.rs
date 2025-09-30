use super::SourceIter;
use crate::{
    entity::Entity,
    query::{QueryData, QueryFilter},
    relationship::{Relationship, RelationshipTarget},
    system::Query,
};
use alloc::collections::VecDeque;
use smallvec::SmallVec;

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
    ) -> DescendantIter<BreadthFirst<'w, 's, D, F, S>>
    where
        D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
    {
        DescendantIter(BreadthFirst::new(self, entity))
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
    ) -> DescendantIter<DepthFirst<'w, 's, D, F, S>>
    where
        D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
        SourceIter<'w, S>: DoubleEndedIterator,
    {
        DescendantIter(DepthFirst::new(self, entity))
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

/// A [`DescendantsTraversal`] of [`Entity`]s over the descendants of an [`Entity`].
///
/// Traverses the hierarchy breadth-first.
pub struct BreadthFirst<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipTarget>
where
    D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
{
    children_query: &'w Query<'w, 's, D, F>,
    vecdeque: VecDeque<Entity>,
}

impl<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipTarget> BreadthFirst<'w, 's, D, F, S>
where
    D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
{
    /// Returns a new [`DescendantIter`].
    fn new(children_query: &'w Query<'w, 's, D, F>, entity: Entity) -> Self {
        BreadthFirst {
            children_query,
            vecdeque: children_query
                .get(entity)
                .into_iter()
                .flat_map(RelationshipTarget::iter)
                .collect(),
        }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipTarget> DescendantsTraversal
    for BreadthFirst<'w, 's, D, F, S>
where
    D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
{
    fn next_root(&mut self) -> Option<Entity> {
        self.vecdeque.pop_front()
    }

    fn set_children(&mut self, root: Entity) {
        let Ok(children) = self.children_query.get(root) else {
            return;
        };

        self.vecdeque.extend(children.iter());
    }
}

/// A [`DescendantsTraversal`] of [`Entity`]s over the descendants of an [`Entity`].
///
/// Traverses the hierarchy depth-first.
pub struct DepthFirst<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipTarget>
where
    D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
{
    children_query: &'w Query<'w, 's, D, F>,
    stack: SmallVec<[Entity; 8]>,
}

impl<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipTarget> DepthFirst<'w, 's, D, F, S>
where
    D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
    SourceIter<'w, S>: DoubleEndedIterator,
{
    /// Returns a new [`DescendantDepthFirstIter`].
    fn new(children_query: &'w Query<'w, 's, D, F>, entity: Entity) -> Self {
        DepthFirst {
            children_query,
            stack: children_query
                .get(entity)
                .map_or(SmallVec::new(), |children| children.iter().rev().collect()),
        }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipTarget> DescendantsTraversal
    for DepthFirst<'w, 's, D, F, S>
where
    D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
    SourceIter<'w, S>: DoubleEndedIterator,
{
    fn next_root(&mut self) -> Option<Entity> {
        self.stack.pop()
    }

    fn set_children(&mut self, root: Entity) {
        let Ok(children) = self.children_query.get(root) else {
            return;
        };

        self.stack.extend(children.iter().rev());
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
/// If all sub hierarchies are linear (only one child) then this yields the same as [`MapWhile`](core::iter::MapWhile).
pub struct FilterHierarchies<T, F> {
    iter: T,
    hierarchy_filter: F,
}

impl<T, F> Iterator for FilterHierarchies<T, F>
where
    T: DescendantsTraversal,
    F: FnMut(&Entity) -> bool,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        let mut root = self.iter.next_root()?;

        while !(self.hierarchy_filter)(&root) {
            root = self.iter.next_root()?;
        }
        self.iter.set_children(root);

        Some(root)
    }
}

/// A [`Iterator`] of [`Entity`]s over the descendants of an [`Entity`].
///
/// Concrete traversal strategy depends on the `Traversal` type.
pub struct DescendantIter<Traversal>(Traversal);

impl<Traversal> DescendantIter<Traversal> {
    /// Creates an iterator which uses a closure to determine if recursive [`RelationshipTarget`]s
    /// should be yielded.
    ///
    /// Once the the provided closure returns `false` for an [`Entity`] it and its recursive
    /// [`RelationshipTarget`]s will not be yielded, effectively skipping that sub hierarchy.
    pub fn filter_hierarchies<HF>(self, filter: HF) -> FilterHierarchies<Self, HF>
    where
        HF: FnMut(&Entity) -> bool,
    {
        FilterHierarchies {
            iter: self,
            hierarchy_filter: filter,
        }
    }
}

impl<Traversal> Iterator for DescendantIter<Traversal>
where
    Traversal: DescendantsTraversal,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        let next_root = self.0.next_root()?;
        self.0.set_children(next_root);

        Some(next_root)
    }
}

impl<Traversal> DescendantsTraversal for DescendantIter<Traversal>
where
    Traversal: DescendantsTraversal,
{
    fn next_root(&mut self) -> Option<Entity> {
        self.0.next_root()
    }

    fn set_children(&mut self, root: Entity) {
        self.0.set_children(root);
    }
}

/// A trait to implement a concrete descendant traversal strategy
///
/// Used to streamline breadth-first and depth-first iteration
trait DescendantsTraversal {
    fn next_root(&mut self) -> Option<Entity>;
    fn set_children(&mut self, root: Entity);
}

#[cfg(test)]
mod test_iter_descendants {
    use crate::{
        prelude::*,
        system::{RunSystemError, RunSystemOnce},
    };
    use alloc::{vec, vec::Vec};

    mod iter_descendants_breadth_first {
        use super::*;

        #[test]
        fn iter_all() -> Result<(), RunSystemError> {
            let mut world = World::new();
            let root = world.spawn_empty().id();
            let a = world.spawn(ChildOf(root)).id();
            let aa = world.spawn(ChildOf(a)).id();
            let ab = world.spawn(ChildOf(a)).id();
            let b = world.spawn(ChildOf(root)).id();
            let ba = world.spawn(ChildOf(b)).id();
            let bb = world.spawn(ChildOf(b)).id();

            let descendants = world.run_system_once(move |q: Query<&Children>| {
                q.iter_descendants(root).collect::<Vec<_>>()
            })?;

            assert_eq!(descendants, vec![a, b, aa, ab, ba, bb]);
            Ok(())
        }
    }

    mod iter_descendants_depth_first {
        use super::*;

        #[test]
        fn iter_all() -> Result<(), RunSystemError> {
            let mut world = World::new();
            let root = world.spawn_empty().id();
            let a = world.spawn(ChildOf(root)).id();
            let aa = world.spawn(ChildOf(a)).id();
            let ab = world.spawn(ChildOf(a)).id();
            let b = world.spawn(ChildOf(root)).id();
            let ba = world.spawn(ChildOf(b)).id();
            let bb = world.spawn(ChildOf(b)).id();

            let descendants = world.run_system_once(move |q: Query<&Children>| {
                q.iter_descendants_depth_first(root).collect::<Vec<_>>()
            })?;

            assert_eq!(descendants, vec![a, aa, ab, b, ba, bb]);
            Ok(())
        }
    }

    mod filter_hierarchies {
        use super::*;

        #[test]
        fn iter_all() -> Result<(), RunSystemError> {
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
        fn skip_entity_when_flat() -> Result<(), RunSystemError> {
            let mut world = World::new();
            let root = world.spawn_empty().id();
            let a = world.spawn(ChildOf(root)).id();
            let skip = world.spawn(ChildOf(root)).id();
            let b = world.spawn(ChildOf(root)).id();

            let descendants = world.run_system_once(move |q: Query<&Children>| {
                q.iter_descendants(root)
                    .filter_hierarchies(|e| e != &skip)
                    .collect::<Vec<_>>()
            })?;

            assert_eq!(descendants, vec![a, b]);
            Ok(())
        }

        #[test]
        fn skip_sub_hierarchy() -> Result<(), RunSystemError> {
            let mut world = World::new();
            let root = world.spawn_empty().id();
            let a = world.spawn(ChildOf(root)).id();
            let skip = world.spawn((ChildOf(root), children![(), ()])).id();
            let b = world.spawn(ChildOf(root)).id();

            let descendants = world.run_system_once(move |q: Query<&Children>| {
                q.iter_descendants(root)
                    .filter_hierarchies(|e| e != &skip)
                    .collect::<Vec<_>>()
            })?;

            assert_eq!(descendants, vec![a, b]);
            Ok(())
        }
    }
}
