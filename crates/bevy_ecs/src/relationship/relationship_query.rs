use super::SourceIter;
use crate::{
    entity::Entity,
    query::{QueryData, QueryFilter},
    relationship::{Relationship, RelationshipTarget},
    system::Query,
};
use alloc::collections::VecDeque;
use core::marker::PhantomData;
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
    /// [`RelationshipTarget`] in breadth-first order.
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

/// An iteration strategy of [`Entity`]s over the descendants of an [`Entity`].
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
    fn new(children_query: &'w Query<'w, 's, D, F>, entity: Entity) -> Self {
        Self {
            children_query,
            vecdeque: children_query
                .get(entity)
                .into_iter()
                .flat_map(RelationshipTarget::iter)
                .collect(),
        }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipTarget> DescendantsIterator
    for BreadthFirst<'w, 's, D, F, S>
where
    D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
{
    fn next_node(&mut self) -> Option<Entity> {
        self.vecdeque.pop_front()
    }

    fn set_children(&mut self, node: Entity) {
        let Ok(children) = self.children_query.get(node) else {
            return;
        };

        self.vecdeque.extend(children.iter());
    }
}

/// An iteration strategy of [`Entity`]s over the descendants of an [`Entity`].
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
    fn new(children_query: &'w Query<'w, 's, D, F>, entity: Entity) -> Self {
        Self {
            children_query,
            stack: children_query
                .get(entity)
                .map_or(SmallVec::new(), |children| children.iter().rev().collect()),
        }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipTarget> DescendantsIterator
    for DepthFirst<'w, 's, D, F, S>
where
    D::ReadOnly: QueryData<Item<'w, 's> = &'w S>,
    SourceIter<'w, S>: DoubleEndedIterator,
{
    fn next_node(&mut self) -> Option<Entity> {
        self.stack.pop()
    }

    fn set_children(&mut self, node: Entity) {
        let Ok(children) = self.children_query.get(node) else {
            return;
        };

        self.stack.extend(children.iter().rev());
    }
}

/// An [`Iterator`] of [`Entity`]s over the descendants of an [`Entity`].
///
/// Concrete traversal strategy depends on the `Traversal` type.
pub struct DescendantIter<Traversal>(Traversal);

impl<Traversal> DescendantIter<Traversal> {
    /// Creates an iterator which uses a closure to determine if recursive [`RelationshipTarget`]s
    /// should be yielded.
    ///
    /// Once the provided closure returns `false` for an [`Entity`] it and its recursive
    /// [`RelationshipTarget`]s will not be yielded, effectively skipping the sub hierarchy where
    /// that [`Entity`] is the root.
    pub fn filter_hierarchies<F>(self, predicate: F) -> FilterHierarchies<Self, F>
    where
        F: FnMut(&Entity) -> bool,
    {
        FilterHierarchies {
            iter: self,
            predicate,
        }
    }

    /// Creates an iterator which uses a closure to both filter and map over recursive
    /// [`RelationshipTarget`]s.
    ///
    /// Once the provided closure returns `None` for an [`Entity`] the mapped values for
    /// it and its recursive [`RelationshipTarget`]s will not be yielded, effectively skipping the
    /// sub hierarchy where that [`Entity`] is the root.
    pub fn filter_map_hierarchies<F, R>(self, map: F) -> FilterMapHierarchies<Self, F, R>
    where
        F: FnMut(Entity) -> Option<R>,
    {
        FilterMapHierarchies {
            iter: self,
            map,
            _p: PhantomData,
        }
    }
}

impl<Traversal> Iterator for DescendantIter<Traversal>
where
    Traversal: DescendantsIterator,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        let next_root = self.0.next_node()?;
        self.0.set_children(next_root);

        Some(next_root)
    }
}

impl<Traversal> DescendantsIterator for DescendantIter<Traversal>
where
    Traversal: DescendantsIterator,
{
    fn next_node(&mut self) -> Option<Entity> {
        self.0.next_node()
    }

    fn set_children(&mut self, node: Entity) {
        self.0.set_children(node);
    }
}

/// An [`Iterator`] of [`Entity`]s over the descendants of an [`Entity`].
///
/// Allows conditional skipping of sub hierarchies.
pub struct FilterHierarchies<T, F> {
    iter: T,
    predicate: F,
}

impl<T, F> Iterator for FilterHierarchies<T, F>
where
    T: DescendantsIterator,
    F: FnMut(&Entity) -> bool,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        let mut node;

        loop {
            node = self.iter.next_node()?;
            if (self.predicate)(&node) {
                break;
            }
        }
        self.iter.set_children(node);

        Some(node)
    }
}

/// An [`Iterator`] of [`Entity`]s over the descendants of an [`Entity`].
///
/// Allows conditional skipping of sub hierarchies.
pub struct FilterMapHierarchies<T, F, R> {
    iter: T,
    map: F,
    _p: PhantomData<R>,
}

impl<T, F, R> Iterator for FilterMapHierarchies<T, F, R>
where
    T: DescendantsIterator,
    F: FnMut(Entity) -> Option<R>,
{
    type Item = R;

    fn next(&mut self) -> Option<Self::Item> {
        let mut node;
        let mut value;

        loop {
            node = self.iter.next_node()?;
            value = (self.map)(node);
            if value.is_some() {
                break;
            }
        }
        self.iter.set_children(node);

        value
    }
}

/// A trait to implement a concrete descendant traversal strategy
///
/// Used to streamline breadth-first and depth-first iteration
trait DescendantsIterator {
    fn next_node(&mut self) -> Option<Entity>;
    fn set_children(&mut self, node: Entity);
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

            let descendants = world.run_system_once(move |c: Query<&Children>| {
                c.iter_descendants(root).collect::<Vec<_>>()
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

            let descendants = world.run_system_once(move |c: Query<&Children>| {
                c.iter_descendants_depth_first(root).collect::<Vec<_>>()
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

            let descendants = world.run_system_once(move |c: Query<&Children>| {
                c.iter_descendants(root)
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

            let descendants = world.run_system_once(move |c: Query<&Children>| {
                c.iter_descendants(root)
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

            let descendants = world.run_system_once(move |c: Query<&Children>| {
                c.iter_descendants(root)
                    .filter_hierarchies(|e| e != &skip)
                    .collect::<Vec<_>>()
            })?;

            assert_eq!(descendants, vec![a, b]);
            Ok(())
        }
    }

    mod map_hierarchies {
        use super::*;

        #[test]
        fn iter_all() -> Result<(), RunSystemError> {
            let mut world = World::new();
            let root = world
                .spawn(children![Name::from("a"), Name::from("b"), Name::from("c")])
                .id();

            let names = world.run_system_once(move |c: Query<&Children>, n: Query<&Name>| {
                c.iter_descendants(root)
                    .filter_map_hierarchies(|e| n.get(e).ok().cloned())
                    .collect::<Vec<_>>()
            })?;

            assert_eq!(
                names,
                vec![Name::from("a"), Name::from("b"), Name::from("c")]
            );
            Ok(())
        }

        #[test]
        fn skip_entity_when_flat() -> Result<(), RunSystemError> {
            let mut world = World::new();
            let root = world
                .spawn(children![
                    Name::from("a"),
                    Name::from("skip"),
                    Name::from("b"),
                ])
                .id();

            let names = world.run_system_once(move |c: Query<&Children>, n: Query<&Name>| {
                c.iter_descendants(root)
                    .filter_map_hierarchies(|e| match n.get(e) {
                        Ok(name) if name.as_str() != "skip" => Some(name.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            })?;

            assert_eq!(names, vec![Name::from("a"), Name::from("b")]);
            Ok(())
        }

        #[test]
        fn skip_sub_hierarchy() -> Result<(), RunSystemError> {
            let mut world = World::new();
            let root = world
                .spawn(children![
                    Name::from("a"),
                    (
                        Name::from("skip"),
                        children![Name::from("skip child a"), Name::from("skip child b")]
                    ),
                    Name::from("b"),
                ])
                .id();

            let names = world.run_system_once(move |c: Query<&Children>, n: Query<&Name>| {
                c.iter_descendants(root)
                    .filter_map_hierarchies(|e| match n.get(e) {
                        Ok(name) if name.as_str() != "skip" => Some(name.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            })?;

            assert_eq!(names, vec![Name::from("a"), Name::from("b")]);
            Ok(())
        }
    }
}
