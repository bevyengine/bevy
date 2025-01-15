// TODO: remove this
#![allow(missing_docs)]

pub use bevy_ecs_macros::{Relationship, RelationshipSources};

use crate::{
    bundle::Bundle,
    component::{Component, ComponentId, Mutable},
    entity::Entity,
    query::{QueryData, QueryFilter, WorldQuery},
    system::{
        command::HandleError,
        entity_command::{self, CommandWithEntity},
        error_handler, Commands, EntityCommands, Query,
    },
    world::{DeferredWorld, EntityWorldMut, World},
};
use alloc::{collections::VecDeque, vec::Vec};
use core::marker::PhantomData;
use log::warn;
use smallvec::SmallVec;

// The "deprecated" state is used to prevent users from mutating the internal RelationshipSource collection.
// These internals are allowed to modify the internal RelationshipSource collection.
#[allow(deprecated)]
pub trait Relationship: Component + Sized {
    type RelationshipSources: RelationshipSources<Relationship = Self>;
    /// Gets the [`Entity`] ID of the related entity.
    fn get(&self) -> Entity;
    fn set(&mut self, entity: Entity);
    fn from(entity: Entity) -> Self;
    fn on_insert(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
        let parent = world.entity(entity).get::<Self>().unwrap().get();
        if parent == entity {
            warn!(
                "The {}({parent:?}) relationship on entity {entity:?} points to itself. The invalid {} relationship has been removed.",
                core::any::type_name::<Self>(),
                core::any::type_name::<Self>()
            );
            world.commands().entity(entity).remove::<Self>();
        }
        if let Ok(mut parent_entity) = world.get_entity_mut(parent) {
            if let Some(mut relationship_sources) =
                parent_entity.get_mut::<Self::RelationshipSources>()
            {
                relationship_sources.collection_mut().add(entity);
            } else {
                let mut sources =
                    <Self::RelationshipSources as RelationshipSources>::with_capacity(1);
                sources.collection_mut().add(entity);
                world.commands().entity(parent).insert(sources);
            }
        } else {
            warn!(
                "The {}({parent:?}) relationship on entity {entity:?} relates to an entity that does not exist. The invalid {} relationship has been removed.",
                core::any::type_name::<Self>(),
                core::any::type_name::<Self>()
            );
            world.commands().entity(entity).remove::<Self>();
        }
    }

    // note: think of this as "on_drop"
    fn on_replace(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
        let parent = world.entity(entity).get::<Self>().unwrap().get();
        if let Ok(mut parent_entity) = world.get_entity_mut(parent) {
            if let Some(mut relationship_sources) =
                parent_entity.get_mut::<Self::RelationshipSources>()
            {
                relationship_sources.collection_mut().remove(entity);
                if relationship_sources.len() == 0 {
                    if let Some(mut entity) = world.commands().get_entity(parent) {
                        entity.remove::<Self::RelationshipSources>();
                    }
                }
            }
        }
    }
}

// The "deprecated" state is used to prevent users from mutating the internal RelationshipSource collection.
// These internals are allowed to modify the internal RelationshipSource collection.
#[allow(deprecated)]
pub trait RelationshipSources: Component<Mutability = Mutable> + Sized {
    type Relationship: Relationship<RelationshipSources = Self>;
    type Collection: RelationshipSourceCollection;

    fn collection(&self) -> &Self::Collection;
    #[deprecated = "Modifying the internal RelationshipSource collection should only be done by internals as it can invalidate relationships."]
    fn collection_mut(&mut self) -> &mut Self::Collection;
    #[deprecated = "Creating a relationship source manually should only be done by internals as it can invalidate relationships."]
    fn from_collection(collection: Self::Collection) -> Self;

    fn on_replace(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
        // NOTE: this unsafe code is an optimization. We could make this safe, but it would require
        // copying the RelationshipSources collection
        // SAFETY: This only reads the Self component and queues Remove commands
        unsafe {
            let world = world.as_unsafe_world_cell();
            let sources = world.get_entity(entity).unwrap().get::<Self>().unwrap();
            let mut commands = world.get_raw_command_queue();
            for source_entity in sources.iter() {
                if world.get_entity(source_entity).is_some() {
                    commands.push(
                        entity_command::remove::<Self::Relationship>()
                            .with_entity(source_entity)
                            .handle_error_with(error_handler::silent()),
                    );
                } else {
                    warn!("Tried to despawn non-existent entity {}", source_entity);
                }
            }
        }
    }

    fn on_despawn(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
        // NOTE: this unsafe code is an optimization. We could make this safe, but it would require
        // copying the RelationshipSources collection
        // SAFETY: This only reads the Self component and queues despawn commands
        unsafe {
            let world = world.as_unsafe_world_cell();
            let sources = world.get_entity(entity).unwrap().get::<Self>().unwrap();
            let mut commands = world.get_raw_command_queue();
            for source_entity in sources.iter() {
                if world.get_entity(source_entity).is_some() {
                    commands.push(
                        entity_command::despawn()
                            .with_entity(source_entity)
                            .handle_error_with(error_handler::silent()),
                    );
                } else {
                    warn!("Tried to despawn non-existent entity {}", source_entity);
                }
            }
        }
    }

    fn with_capacity(capacity: usize) -> Self {
        let collection =
            <Self::Collection as RelationshipSourceCollection>::with_capacity(capacity);
        Self::from_collection(collection)
    }

    #[inline]
    fn iter(&self) -> impl DoubleEndedIterator<Item = Entity> {
        self.collection().iter()
    }

    #[inline]
    fn len(&self) -> usize {
        self.collection().len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.collection().is_empty()
    }
}

pub trait RelationshipSourceCollection {
    fn with_capacity(capacity: usize) -> Self;
    fn add(&mut self, entity: Entity);
    fn remove(&mut self, entity: Entity);
    fn iter(&self) -> impl DoubleEndedIterator<Item = Entity>;
    fn take(&mut self) -> Vec<Entity>;
    fn len(&self) -> usize;
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl RelationshipSourceCollection for Vec<Entity> {
    fn with_capacity(capacity: usize) -> Self {
        Vec::with_capacity(capacity)
    }

    fn add(&mut self, entity: Entity) {
        Vec::push(self, entity);
    }

    fn remove(&mut self, entity: Entity) {
        if let Some(index) = <[Entity]>::iter(self).position(|e| *e == entity) {
            Vec::remove(self, index);
        }
    }

    fn iter(&self) -> impl DoubleEndedIterator<Item = Entity> {
        <[Entity]>::iter(self).copied()
    }

    fn take(&mut self) -> Vec<Entity> {
        core::mem::take(self)
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }
}

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

pub struct RelatedSpawner<'w, R: Relationship> {
    target: Entity,
    world: &'w mut World,
    _marker: PhantomData<R>,
}

impl<'w, R: Relationship> RelatedSpawner<'w, R> {
    pub fn new(world: &'w mut World, target: Entity) -> Self {
        Self {
            world,
            target,
            _marker: PhantomData,
        }
    }

    pub fn spawn(&mut self, bundle: impl Bundle) -> EntityWorldMut<'_> {
        self.world.spawn((R::from(self.target), bundle))
    }

    pub fn spawn_empty(&mut self) -> EntityWorldMut<'_> {
        self.world.spawn(R::from(self.target))
    }

    pub fn target_entity(&self) -> Entity {
        self.target
    }
}

pub struct RelatedSpawnerCommands<'w, R: Relationship> {
    target: Entity,
    commands: Commands<'w, 'w>,
    _marker: PhantomData<R>,
}

impl<'w, R: Relationship> RelatedSpawnerCommands<'w, R> {
    pub fn new(commands: Commands<'w, 'w>, target: Entity) -> Self {
        Self {
            commands,
            target,
            _marker: PhantomData,
        }
    }
    pub fn spawn(&mut self, bundle: impl Bundle) -> EntityCommands<'_> {
        self.commands.spawn((R::from(self.target), bundle))
    }

    pub fn spawn_empty(&mut self) -> EntityCommands<'_> {
        self.commands.spawn(R::from(self.target))
    }

    pub fn target_entity(&self) -> Entity {
        self.target
    }
}

impl<'w> EntityWorldMut<'w> {
    /// Spawns entities related to this entity (with the `R` relationship) by taking a function that operates on a [`RelatedSpawner`].
    pub fn with_related<R: Relationship>(
        &mut self,
        func: impl FnOnce(&mut RelatedSpawner<R>),
    ) -> &mut Self {
        let parent = self.id();
        self.world_scope(|world| {
            func(&mut RelatedSpawner::new(world, parent));
        });
        self
    }

    /// Relates the given entities to this entity with the relation `R`
    pub fn add_related<R: Relationship>(&mut self, related: &[Entity]) -> &mut Self {
        let id = self.id();
        self.world_scope(|world| {
            for related in related {
                world.entity_mut(*related).insert(R::from(id));
            }
        });
        self
    }
}

impl<'a> EntityCommands<'a> {
    /// Spawns entities related to this entity (with the `R` relationship) by taking a function that operates on a [`RelatedSpawner`].
    pub fn with_related<R: Relationship>(
        &mut self,
        func: impl FnOnce(&mut RelatedSpawnerCommands<R>),
    ) -> &mut Self {
        let id = self.id();
        func(&mut RelatedSpawnerCommands::new(self.commands(), id));
        self
    }

    /// Relates the given entities to this entity with the relation `R`
    pub fn add_related<R: Relationship>(&mut self, related: &[Entity]) -> &mut Self {
        let id = self.id();
        let related = related.to_vec();
        self.commands().queue(move |world: &mut World| {
            for related in related {
                world.entity_mut(related).insert(R::from(id));
            }
        });
        self
    }
}

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use crate::world::World;
    use crate::{
        entity::Entity,
        relationship::{Relationship, RelationshipSources},
    };
    use alloc::vec::Vec;

    #[test]
    fn custom_relationship() {
        #[derive(Relationship)]
        #[relationship_sources(LikedBy)]
        struct Likes(pub Entity);

        #[derive(RelationshipSources)]
        #[relationship(Likes)]
        struct LikedBy(Vec<Entity>);

        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn(Likes(a)).id();
        let c = world.spawn(Likes(a)).id();
        assert_eq!(world.entity(a).get::<LikedBy>().unwrap().0, &[b, c]);
    }
}
