use core::marker::PhantomData;

use crate::{
    archetype::Archetype,
    component::Tick,
    entity::Entity,
    query::{QueryData, QueryFilter, WorldQuery},
    relationship::{Relationship, RelationshipTarget},
    storage::{Table, TableRow},
    system::Query,
    world::unsafe_world_cell::UnsafeWorldCell,
};
use alloc::collections::VecDeque;
use smallvec::SmallVec;

use super::SourceIter;

impl<'w, 's, D: QueryData, F: QueryFilter> Query<'w, 's, D, F> {
    /// If the given `entity` contains the `R` [`Relationship`] component, returns the
    /// target entity of that relationship.
    pub fn related<R: Relationship>(&'w self, entity: Entity) -> Option<Entity>
    where
        <D as QueryData>::ReadOnly: WorldQuery<Item<'w> = &'w R>,
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
        <D as QueryData>::ReadOnly: WorldQuery<Item<'w> = &'w S>,
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
        <D as QueryData>::ReadOnly: WorldQuery<Item<'w> = &'w R>,
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
    ) -> impl Iterator<Item = Entity> + 'w
    where
        <D as QueryData>::ReadOnly: WorldQuery<Item<'w> = &'w S>,
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
        D::ReadOnly: WorldQuery<Item<'w> = (Option<&'w R>, Option<&'w R::RelationshipTarget>)>,
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
        D::ReadOnly: WorldQuery<Item<'w> = &'w S>,
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
        D::ReadOnly: WorldQuery<Item<'w> = &'w S>,
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
        D::ReadOnly: WorldQuery<Item<'w> = &'w R>,
    {
        AncestorIter::new(self, entity)
    }
}

/// An [`Iterator`] of [`Entity`]s over the descendants of an [`Entity`].
///
/// Traverses the hierarchy breadth-first.
pub struct DescendantIter<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipTarget>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w S>,
{
    children_query: &'w Query<'w, 's, D, F>,
    vecdeque: VecDeque<Entity>,
}

impl<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipTarget> DescendantIter<'w, 's, D, F, S>
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
                .flat_map(RelationshipTarget::iter)
                .collect(),
        }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipTarget> Iterator
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
pub struct DescendantDepthFirstIter<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipTarget>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w S>,
{
    children_query: &'w Query<'w, 's, D, F>,
    stack: SmallVec<[Entity; 8]>,
}

impl<'w, 's, D: QueryData, F: QueryFilter, S: RelationshipTarget>
    DescendantDepthFirstIter<'w, 's, D, F, S>
where
    D::ReadOnly: WorldQuery<Item<'w> = &'w S>,
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
    D::ReadOnly: WorldQuery<Item<'w> = &'w S>,
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

/// A [`QueryFilter`] type that filters for entities that are related via `R` to an entity that matches `F`.
///
/// This works by looking up the related entity using the `R` relationship component,
/// then checking if that related entity matches the filter given in `F`.
///
/// # Examples
///
/// ```rust
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::system::RunSystemOnce;
///
/// #[derive(Component)]
/// struct A;
///
/// let mut world = World::new();
/// let parent = world.spawn(A).id();
/// let child = world.spawn(ChildOf(parent))).id();
/// let unrelated = world.spawn_empty().id();
/// let grandchild = world.spawn(ChildOf(child)).id();
///
/// fn iterate_related_to_a(query: Query<Entity, RelatedTo<ChildOf, With<A>>>) {
///     for entity in query.iter() {
///        // Only the child entity should be iterated;
///        // the parent, unrelated and chrandchild entities should be skipped,
///        // as they are not related to an entity with the `A` component.
///        assert_eq!(entity, child);
///    }
/// }
///
/// world.run_system_once(iterate_related_to_a);
/// ```
pub struct RelatedTo<R: Relationship, F: QueryFilter> {
    _relationship: PhantomData<R>,
    _filter: PhantomData<F>,
}

unsafe impl<R: Relationship, F: QueryFilter> WorldQuery for RelatedTo<R, F> {
    type Item<'a> = <F as WorldQuery>::Item<'a>;

    type Fetch<'a> = RelatedToFetch<'a, R, F>;

    type State = RelatedToState<R, F>;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
        <F as WorldQuery>::shrink(item)
    }

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
        RelatedToFetch {
            relation_fetch: <&'static R as WorldQuery>::shrink_fetch(fetch.relation_fetch),
            filter_fetch: <F as WorldQuery>::shrink_fetch(fetch.filter_fetch),
        }
    }

    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        state: &Self::State,
        last_run: Tick,
        this_run: Tick,
    ) -> Self::Fetch<'w> {
        RelatedToFetch {
            relation_fetch: <&'static R as WorldQuery>::init_fetch(
                world,
                &state.relation_state,
                last_run,
                this_run,
            ),
            filter_fetch: <F as WorldQuery>::init_fetch(
                world,
                &state.filter_state,
                last_run,
                this_run,
            ),
        }
    }

    const IS_DENSE: bool = <F as WorldQuery>::IS_DENSE & <&R as WorldQuery>::IS_DENSE;

    unsafe fn set_archetype<'w>(
        fetch: &mut Self::Fetch<'w>,
        state: &Self::State,
        archetype: &'w Archetype,
        table: &'w Table,
    ) {
        <&'static R as WorldQuery>::set_archetype(
            &mut fetch.relation_fetch,
            &state.relation_state,
            archetype,
            table,
        );
        <F as WorldQuery>::set_archetype(
            &mut fetch.filter_fetch,
            &state.filter_state,
            archetype,
            table,
        );
    }

    unsafe fn set_table<'w>(fetch: &mut Self::Fetch<'w>, state: &Self::State, table: &'w Table) {
        <&'static R as WorldQuery>::set_table(
            &mut fetch.relation_fetch,
            &state.relation_state,
            table,
        );
        <F as WorldQuery>::set_table(&mut fetch.filter_fetch, &state.filter_state, table);
    }

    unsafe fn fetch<'w>(
        fetch: &mut Self::Fetch<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        // Look up the relationship
        let relation =
            <&'static R as WorldQuery>::fetch(&mut fetch.relation_fetch, entity, table_row);
        // Then figure out what the related entity is
        let related_entity = relation.get();

        // Finally, check if the related entity matches the filter
        <F as WorldQuery>::fetch(&mut fetch.filter_fetch, related_entity, table_row)
    }

    fn update_component_access(
        state: &Self::State,
        access: &mut crate::query::FilteredAccess<crate::component::ComponentId>,
    ) {
        <&'static R as WorldQuery>::update_component_access(&state.relation_state, access);
        <F as WorldQuery>::update_component_access(&state.filter_state, access);
    }

    fn init_state(world: &mut crate::prelude::World) -> Self::State {
        RelatedToState {
            relation_state: <&'static R as WorldQuery>::init_state(world),
            filter_state: <F as WorldQuery>::init_state(world),
        }
    }

    fn get_state(components: &crate::component::Components) -> Option<Self::State> {
        Some(RelatedToState {
            relation_state: <&'static R as WorldQuery>::get_state(components)?,
            filter_state: <F as WorldQuery>::get_state(components)?,
        })
    }

    fn matches_component_set(
        state: &Self::State,
        set_contains_id: &impl Fn(crate::component::ComponentId) -> bool,
    ) -> bool {
        // We need to look at both the relationship and the filter components,
        // but they do not need to be on the same entity.
        // As a result, we use an OR operation, rather than the AND operation used in other WorldQuery implementations.
        <&'static R as WorldQuery>::matches_component_set(&state.relation_state, set_contains_id)
            || <F as WorldQuery>::matches_component_set(&state.filter_state, set_contains_id)
    }
}

unsafe impl<R: Relationship, F: QueryFilter> QueryFilter for RelatedTo<R, F> {
    // The information about whether or not a related entity matches the filter
    // varies between entities found in the same archetype,
    // so rapidly pre-computing the length of the filtered set is not possible.
    const IS_ARCHETYPAL: bool = false;

    unsafe fn filter_fetch(
        fetch: &mut Self::Fetch<'_>,
        entity: Entity,
        table_row: TableRow,
    ) -> bool {
        // First, look up the relationship
        // SAFETY: the caller promises that we only call this method after WorldQuery::set_table or WorldQuery::set_archetype,
        // and that the entity and table_row are in the range of the current table and archetype.
        // No simultaneous conflicting component accesses exist, as both parts of the filter are read-only.
        let relation = unsafe {
            <&'static R as WorldQuery>::fetch(&mut fetch.relation_fetch, entity, table_row)
        };

        // Then figure out what the related entity is
        let related_entity = relation.get();

        // Finally, check if the related entity matches the filter
        // SAFETY: the safety invariants for calling `filter_fetch` on `F` are upheld by the caller,
        // as they are the same as the safety invariants for calling this method
        unsafe {
            <F as QueryFilter>::filter_fetch(&mut fetch.filter_fetch, related_entity, table_row)
        }
    }
}

/// The [`WorldQuery::Fetch`] type for [`RelatedTo`].
///
/// This is used internally to implement [`WorldQuery`] for [`RelatedTo`].
pub struct RelatedToFetch<'w, R: Relationship, F: QueryFilter> {
    /// The fetch for the relationship component,
    /// used to look up the target entity.
    relation_fetch: <&'static R as WorldQuery>::Fetch<'w>,
    /// The fetch for the filter component,
    /// used to determine if the target entity matches the filter.
    filter_fetch: <F as WorldQuery>::Fetch<'w>,
}

impl<'w, R: Relationship, F: QueryFilter> Clone for RelatedToFetch<'w, R, F> {
    fn clone(&self) -> Self {
        Self {
            relation_fetch: self.relation_fetch.clone(),
            filter_fetch: self.filter_fetch.clone(),
        }
    }
}

/// The [`WorldQuery::State`] type for [`RelatedTo`].
///
/// This is used internally to implement [`WorldQuery`] for [`RelatedTo`].
pub struct RelatedToState<R: Relationship, F: QueryFilter> {
    /// The state for the relationship component,
    /// used to look up the target entity.
    relation_state: <&'static R as WorldQuery>::State,
    /// The state for the filter component,
    /// used to determine if the target entity matches the filter.
    filter_state: <F as WorldQuery>::State,
}

#[cfg(test)]
mod tests {
    use super::RelatedTo;
    use crate as bevy_ecs;
    use crate::prelude::{Changed, ChildOf, Component, Entity, With, Without, World};

    #[derive(Component)]
    struct A;

    #[test]
    fn related_to_empty_filter() {
        let mut world = World::default();
        let parent = world.spawn_empty().id();
        let child = world.spawn(ChildOf(parent)).id();
        let _unrelated = world.spawn_empty().id();
        let grandchild = world.spawn(ChildOf(child)).id();

        let mut query_state = world.query_filtered::<Entity, RelatedTo<ChildOf, ()>>();
        for matching_entity in query_state.iter(&world) {
            let matches_child_or_grandchild =
                matching_entity == child || matching_entity == grandchild;
            assert!(
                matches_child_or_grandchild,
                "Entity {matching_entity} should have a parent"
            );
        }

        assert_eq!(query_state.iter(&world).count(), 2);
    }

    #[test]
    fn related_to_with() {
        let mut world = World::default();
        let parent = world.spawn(A).id();
        let child = world.spawn(ChildOf(parent)).id();
        let mut query_state = world.query_filtered::<Entity, RelatedTo<ChildOf, With<A>>>();
        let fetched_child = query_state.iter(&world).next().unwrap();

        assert_eq!(child, fetched_child);
    }

    #[test]
    fn related_to_changed() {
        let mut world = World::default();
        let parent = world.spawn(A).id();
        let child = world.spawn(ChildOf(parent)).id();
        // Changed is true when entities are first added, so this should match
        let mut query_state = world.query_filtered::<Entity, RelatedTo<ChildOf, Changed<A>>>();
        let fetched_child = query_state.iter(&world).next().unwrap();

        assert_eq!(child, fetched_child);
    }

    #[test]
    fn related_to_without() {
        let mut world = World::default();
        let parent = world.spawn_empty().id();
        let child = world.spawn(ChildOf(parent)).id();
        let mut query_state = world.query_filtered::<Entity, RelatedTo<ChildOf, Without<A>>>();
        let fetched_child = query_state.iter(&world).next().unwrap();

        assert_eq!(child, fetched_child);
    }

    #[test]
    fn related_to_impossible_filter() {
        let mut world = World::default();
        let parent = world.spawn_empty().id();
        let child = world.spawn(ChildOf(parent)).id();
        // No entity could possibly match this filter:
        // it requires entities to both have and not have the `A` component.
        let mut query_state =
            world.query_filtered::<Entity, RelatedTo<ChildOf, (With<A>, Without<A>)>>();
        let maybe_fetched_child = query_state.get(&world, child);

        assert!(maybe_fetched_child.is_err());
    }
}
