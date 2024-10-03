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
use core::fmt::Debug;
use core::marker::PhantomData;
use core::ops::Deref;
use smallvec::SmallVec;

use super::{OneToManyEvent, OneToManyOne};

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
pub struct OneToManyMany<R> {
    entities: SmallVec<[Entity; 8]>,
    #[cfg_attr(feature = "reflect", reflect(ignore))]
    _phantom: PhantomData<fn(&R)>,
}

impl<R: 'static> OneToManyMany<R> {
    fn associate(mut world: DeferredWorld<'_>, a_id: Entity, _component: ComponentId) {
        world.commands().queue(move |world: &mut World| {
            let b_ids_len = world
                .get_entity(a_id)
                .and_then(|a| a.get::<Self>())
                .map(|a_relationship| a_relationship.entities.len());

            let Some(b_ids_len) = b_ids_len else { return };

            for b_id_index in 0..b_ids_len {
                let b = world
                    .get_entity(a_id)
                    .and_then(|a| a.get::<Self>())
                    .map(|a_relationship| a_relationship.entities[b_id_index])
                    .and_then(|b_id| world.get_entity_mut(b_id));

                let Some(mut b) = b else { return };

                let b_id = b.id();

                let b_points_to_a = b
                    .get::<OneToManyOne<R>>()
                    .is_some_and(|b_relationship| b_relationship.get() == a_id);

                if !b_points_to_a {
                    b.insert(OneToManyOne::<R>::new(a_id));

                    if let Some(mut moved) = world.get_resource_mut::<Events<OneToManyEvent<R>>>() {
                        moved.send(OneToManyEvent::<R>::Added(a_id, b_id, PhantomData));
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
                    .and_then(|a| a.get::<Self>())
                    .is_some_and(|a_relationship| a_relationship.entities.contains(&b_id));

                let b_points_to_a = world
                    .get_entity(b_id)
                    .and_then(|b| b.get::<OneToManyOne<R>>())
                    .is_some_and(|b_relationship| b_relationship.get() == a_id);

                if b_points_to_a && !a_points_to_b {
                    if let Some(mut b) = world.get_entity_mut(b_id) {
                        b.remove::<OneToManyOne<R>>();
                    }

                    if let Some(mut moved) = world.get_resource_mut::<Events<OneToManyEvent<R>>>() {
                        moved.send(OneToManyEvent::<R>::Removed(a_id, b_id, PhantomData));
                    }
                }
            }
        });
    }
}

impl<R> PartialEq for OneToManyMany<R> {
    fn eq(&self, other: &Self) -> bool {
        self.entities == other.entities
    }
}

impl<R> Eq for OneToManyMany<R> {}

impl<R> Debug for OneToManyMany<R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("OneToManyMany")
            .field(&self.entities)
            .field(&core::any::type_name::<R>())
            .finish()
    }
}

impl<R> VisitEntitiesMut for OneToManyMany<R> {
    fn visit_entities_mut<F: FnMut(&mut Entity)>(&mut self, mut f: F) {
        for entity in &mut self.entities {
            f(entity);
        }
    }
}

impl<R> Deref for OneToManyMany<R> {
    type Target = [Entity];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.entities
    }
}

impl<'a, R> IntoIterator for &'a OneToManyMany<R> {
    type Item = <Self::IntoIter as Iterator>::Item;

    type IntoIter = core::slice::Iter<'a, Entity>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.entities.iter()
    }
}

impl<R> FromIterator<Entity> for OneToManyMany<R> {
    fn from_iter<T: IntoIterator<Item = Entity>>(iter: T) -> Self {
        Self::from_smallvec(iter.into_iter().collect())
    }
}

impl<R> Default for OneToManyMany<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R> OneToManyMany<R> {
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
}
