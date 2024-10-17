#[cfg(feature = "reflect")]
use bevy_ecs::reflect::{
    ReflectComponent, ReflectFromWorld, ReflectMapEntities, ReflectVisitEntities,
    ReflectVisitEntitiesMut,
};
use bevy_ecs::{
    component::{Component, ComponentId},
    entity::{Entity, VisitEntitiesMut},
    event::Events,
    world::{DeferredWorld, World},
};
use core::{fmt::Debug, marker::PhantomData, ops::Deref};
use smallvec::SmallVec;

use super::{ManyToOne, OneToManyEvent};

/// Represents one half of a one-to-many relationship between an [`Entity`] and some number of other [entities](Entity).
///
/// The type of relationship is denoted by the parameter `R`.
#[derive(Component)]
#[component(
    on_insert = Self::associate,
    on_replace = Self::disassociate,
    on_remove = Self::disassociate
)]
#[cfg_attr(feature = "reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(
    feature = "reflect",
    reflect(
        Component,
        MapEntities,
        VisitEntities,
        VisitEntitiesMut,
        PartialEq,
        Debug,
        FromWorld
    )
)]
pub struct OneToMany<R> {
    entities: SmallVec<[Entity; 8]>,
    #[cfg_attr(feature = "reflect", reflect(ignore))]
    _phantom: PhantomData<fn(&R)>,
}

impl<R: 'static> OneToMany<R> {
    fn associate(mut world: DeferredWorld<'_>, a_id: Entity, _component: ComponentId) {
        world.commands().queue(move |world: &mut World| {
            let b_ids_len = world
                .get_entity(a_id)
                .ok()
                .and_then(|a| a.get::<Self>())
                .map(|a_relationship| a_relationship.entities.len());

            let Some(b_ids_len) = b_ids_len else { return };

            for b_id_index in 0..b_ids_len {
                let b = world
                    .get_entity(a_id)
                    .ok()
                    .and_then(|a| a.get::<Self>())
                    .map(|a_relationship| a_relationship.entities[b_id_index])
                    .and_then(|b_id| world.get_entity_mut(b_id).ok());

                let Some(mut b) = b else { return };

                let b_id = b.id();

                let b_points_to_a = b
                    .get::<ManyToOne<R>>()
                    .is_some_and(|b_relationship| b_relationship.get() == a_id);

                if !b_points_to_a {
                    b.insert(ManyToOne::<R>::new(a_id));

                    if let Some(mut moved) = world.get_resource_mut::<Events<OneToManyEvent<R>>>() {
                        moved.send(OneToManyEvent::<R>::added(a_id, b_id));
                    }
                }
            }
        });
    }

    fn disassociate(mut world: DeferredWorld<'_>, a_id: Entity, _component: ComponentId) {
        let Some(a_relationship) = world.get::<Self>(a_id) else {
            unreachable!("component hook should only be called when component is available");
        };

        // Cloning to allow a user to `take` the component for modification
        let b_ids = a_relationship.entities.clone();

        world.commands().queue(move |world: &mut World| {
            for b_id in b_ids {
                let a_points_to_b = world
                    .get_entity(a_id)
                    .ok()
                    .and_then(|a| a.get::<Self>())
                    .is_some_and(|a_relationship| a_relationship.entities.contains(&b_id));

                let b_points_to_a = world
                    .get_entity(b_id)
                    .ok()
                    .and_then(|b| b.get::<ManyToOne<R>>())
                    .is_some_and(|b_relationship| b_relationship.get() == a_id);

                if b_points_to_a && !a_points_to_b {
                    if let Ok(mut b) = world.get_entity_mut(b_id) {
                        b.remove::<ManyToOne<R>>();
                    }

                    if let Some(mut moved) = world.get_resource_mut::<Events<OneToManyEvent<R>>>() {
                        moved.send(OneToManyEvent::<R>::removed(a_id, b_id));
                    }
                }
            }
        });
    }
}

impl<R> PartialEq for OneToMany<R> {
    fn eq(&self, other: &Self) -> bool {
        self.entities == other.entities
    }
}

impl<R> Eq for OneToMany<R> {}

impl<R> Debug for OneToMany<R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("OneToMany")
            .field(&self.entities)
            .field(&core::any::type_name::<R>())
            .finish()
    }
}

impl<R> VisitEntitiesMut for OneToMany<R> {
    fn visit_entities_mut<F: FnMut(&mut Entity)>(&mut self, mut f: F) {
        for entity in &mut self.entities {
            f(entity);
        }
    }
}

impl<R> Deref for OneToMany<R> {
    type Target = [Entity];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.entities
    }
}

impl<'a, R> IntoIterator for &'a OneToMany<R> {
    type Item = <Self::IntoIter as Iterator>::Item;

    type IntoIter = core::slice::Iter<'a, Entity>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.entities.iter()
    }
}

impl<R> FromIterator<Entity> for OneToMany<R> {
    fn from_iter<T: IntoIterator<Item = Entity>>(iter: T) -> Self {
        Self::from_smallvec(iter.into_iter().collect())
    }
}

impl<R> Default for OneToMany<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R> OneToMany<R> {
    /// Gets the other [`Entity`] as a slice of length 1.
    #[inline(always)]
    pub fn as_slice(&self) -> &[Entity] {
        &self.entities
    }

    /// Create a new relationship.
    #[inline(always)]
    #[must_use]
    pub fn new() -> Self {
        Self::from_smallvec(SmallVec::new())
    }

    #[inline(always)]
    #[must_use]
    fn from_smallvec(entities: SmallVec<[Entity; 8]>) -> Self {
        Self {
            entities,
            _phantom: PhantomData,
        }
    }

    /// Ensures the provided [`Entity`] is present in this relationship.
    #[inline(always)]
    #[must_use]
    pub fn with(mut self, other: Entity) -> Self {
        if !self.entities.contains(&other) {
            self.entities.push(other);
        }
        self
    }

    /// Ensures the provided [`Entity`] is _not_ present in this relationship.
    #[inline(always)]
    #[must_use]
    pub fn without(mut self, other: Entity) -> Self {
        self.entities.retain(|&mut e| e != other);
        self
    }

    pub(super) fn entities_mut(&mut self) -> &mut SmallVec<[Entity; 8]> {
        &mut self.entities
    }

    /// Swaps the entity at `a_index` with the entity at `b_index`.
    #[inline]
    pub fn swap(&mut self, a_index: usize, b_index: usize) {
        self.entities.swap(a_index, b_index);
    }

    /// Sorts entities [stably](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability)
    /// in place using the provided comparator function.
    ///
    /// For the underlying implementation, see [`slice::sort_by`].
    ///
    /// For the unstable version, see [`sort_unstable_by`](OneToMany::sort_unstable_by).
    ///
    /// See also [`sort_by_key`](OneToMany::sort_by_key), [`sort_by_cached_key`](OneToMany::sort_by_cached_key).
    #[inline]
    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&Entity, &Entity) -> core::cmp::Ordering,
    {
        self.entities.sort_by(compare);
    }

    /// Sorts entities [stably](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability)
    /// in place using the provided key extraction function.
    ///
    /// For the underlying implementation, see [`slice::sort_by_key`].
    ///
    /// For the unstable version, see [`sort_unstable_by_key`](OneToMany::sort_unstable_by_key).
    ///
    /// See also [`sort_by`](OneToMany::sort_by), [`sort_by_cached_key`](OneToMany::sort_by_cached_key).
    #[inline]
    pub fn sort_by_key<K, F>(&mut self, compare: F)
    where
        F: FnMut(&Entity) -> K,
        K: Ord,
    {
        self.entities.sort_by_key(compare);
    }

    /// Sorts entities [stably](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability)
    /// in place using the provided key extraction function. Only evaluates each key at most
    /// once per sort, caching the intermediate results in memory.
    ///
    /// For the underlying implementation, see [`slice::sort_by_cached_key`].
    ///
    /// See also [`sort_by`](OneToMany::sort_by), [`sort_by_key`](OneToMany::sort_by_key).
    #[inline]
    pub fn sort_by_cached_key<K, F>(&mut self, compare: F)
    where
        F: FnMut(&Entity) -> K,
        K: Ord,
    {
        self.entities.sort_by_cached_key(compare);
    }

    /// Sorts entities [unstably](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability)
    /// in place using the provided comparator function.
    ///
    /// For the underlying implementation, see [`slice::sort_unstable_by`].
    ///
    /// For the stable version, see [`sort_by`](OneToMany::sort_by).
    ///
    /// See also [`sort_unstable_by_key`](OneToMany::sort_unstable_by_key).
    #[inline]
    pub fn sort_unstable_by<F>(&mut self, compare: F)
    where
        F: FnMut(&Entity, &Entity) -> core::cmp::Ordering,
    {
        self.entities.sort_unstable_by(compare);
    }

    /// Sorts entities [unstably](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability)
    /// in place using the provided key extraction function.
    ///
    /// For the underlying implementation, see [`slice::sort_unstable_by_key`].
    ///
    /// For the stable version, see [`sort_by_key`](OneToMany::sort_by_key).
    ///
    /// See also [`sort_unstable_by`](OneToMany::sort_unstable_by).
    #[inline]
    pub fn sort_unstable_by_key<K, F>(&mut self, compare: F)
    where
        F: FnMut(&Entity) -> K,
        K: Ord,
    {
        self.entities.sort_unstable_by_key(compare);
    }
}
