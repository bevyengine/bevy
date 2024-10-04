#[cfg(feature = "reflect")]
use bevy_ecs::reflect::{
    ReflectComponent, ReflectFromWorld, ReflectMapEntities, ReflectVisitEntities,
    ReflectVisitEntitiesMut,
};
use bevy_ecs::{
    component::{Component, ComponentId},
    entity::{Entity, VisitEntities, VisitEntitiesMut},
    event::Events,
    traversal::Traversal,
    world::{DeferredWorld, FromWorld, World},
};
use core::fmt::Debug;
use core::marker::PhantomData;
use core::ops::Deref;

use super::{OneToManyEvent, OneToManyMany};

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
pub struct OneToManyOne<R> {
    entity: Entity,
    #[cfg_attr(feature = "reflect", reflect(ignore))]
    _phantom: PhantomData<fn(&R)>,
}

impl<R: 'static> OneToManyOne<R> {
    fn associate(mut world: DeferredWorld<'_>, a_id: Entity, _component: ComponentId) {
        world.commands().queue(move |world: &mut World| {
            let b = world
                .get_entity(a_id)
                .and_then(|a| a.get::<Self>())
                .map(|a_relationship| a_relationship.entity)
                .and_then(|b_id| world.get_entity_mut(b_id));

            let Some(mut b) = b else { return };

            let b_id = b.id();

            let b_points_to_a = b
                .get::<OneToManyMany<R>>()
                .is_some_and(|b_relationship| b_relationship.contains(&a_id));

            if !b_points_to_a {
                if let Some(mut component) = b.get_mut::<OneToManyMany<R>>() {
                    component.entities_mut().push(a_id);
                } else {
                    b.insert(OneToManyMany::<R>::new().with(a_id));
                }

                if let Some(mut moved) = world.get_resource_mut::<Events<OneToManyEvent<R>>>() {
                    moved.send(OneToManyEvent::<R>::Added(a_id, b_id, PhantomData));
                }
            }
        });
    }

    fn disassociate(mut world: DeferredWorld<'_>, a_id: Entity, _component: ComponentId) {
        let Some(a_relationship) = world.get::<Self>(a_id) else {
            unreachable!("component hook should only be called when component is available");
        };

        let b_id = a_relationship.entity;

        world.commands().queue(move |world: &mut World| {
            let a_points_to_b = world
                .get_entity(a_id)
                .and_then(|a| a.get::<Self>())
                .is_some_and(|a_relationship| a_relationship.entity == b_id);

            let b_points_to_a = world
                .get_entity(b_id)
                .and_then(|b| b.get::<OneToManyMany<R>>())
                .is_some_and(|b_relationship| b_relationship.contains(&a_id));

            if b_points_to_a && !a_points_to_b {
                if let Some(mut b) = world.get_entity_mut(b_id) {
                    if let Some(mut component) = b.get_mut::<OneToManyMany<R>>() {
                        component.entities_mut().retain(|&mut e| e != a_id);

                        if component.is_empty() {
                            b.remove::<OneToManyMany<R>>();
                        }
                    }
                }

                if let Some(mut moved) = world.get_resource_mut::<Events<OneToManyEvent<R>>>() {
                    moved.send(OneToManyEvent::<R>::Removed(a_id, b_id, PhantomData));
                }
            }
        });
    }
}

impl<R> PartialEq for OneToManyOne<R> {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity
    }
}

impl<R> Eq for OneToManyOne<R> {}

impl<R> Debug for OneToManyOne<R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("OneToManyOne")
            .field(&self.entity)
            .field(&core::any::type_name::<R>())
            .finish()
    }
}

impl<R> VisitEntities for OneToManyOne<R> {
    fn visit_entities<F: FnMut(Entity)>(&self, mut f: F) {
        f(self.entity);
    }
}

impl<R> VisitEntitiesMut for OneToManyOne<R> {
    fn visit_entities_mut<F: FnMut(&mut Entity)>(&mut self, mut f: F) {
        f(&mut self.entity);
    }
}

// TODO: We need to impl either FromWorld or Default so OneToOne<R> can be registered as Reflect.
// This is because Reflect deserialize by creating an instance and apply a patch on top.
// However OneToOne<R> should only ever be set with a real user-defined entity. It's worth looking into
// better ways to handle cases like this.
impl<R> FromWorld for OneToManyOne<R> {
    #[inline(always)]
    fn from_world(_world: &mut World) -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
            _phantom: PhantomData,
        }
    }
}

impl<R> Deref for OneToManyOne<R> {
    type Target = Entity;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.entity
    }
}

/// This provides generalized hierarchy traversal for use in [event propagation].
///
/// `OneToManyOne::traverse` will never form loops in properly-constructed hierarchies.
///
/// [event propagation]: bevy_ecs::observer::Trigger::propagate
impl<R: 'static> Traversal for &OneToManyOne<R> {
    fn traverse(item: Self::Item<'_>) -> Option<Entity> {
        Some(item.entity)
    }
}

impl<R> OneToManyOne<R> {
    /// Gets the [`Entity`] ID of the other member of this one-to-many relationship.
    #[inline(always)]
    pub fn get(&self) -> Entity {
        self.entity
    }

    /// Gets the other [`Entity`] as a slice.
    #[inline(always)]
    pub fn as_slice(&self) -> &[Entity] {
        core::slice::from_ref(&self.entity)
    }

    /// Create a new relationship with the provided [`Entity`].
    #[inline(always)]
    #[must_use]
    pub fn new(other: Entity) -> Self {
        Self {
            entity: other,
            _phantom: PhantomData,
        }
    }
}
