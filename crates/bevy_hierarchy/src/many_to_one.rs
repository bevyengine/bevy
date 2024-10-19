#[cfg(feature = "reflect")]
use bevy_ecs::reflect::{
    ReflectComponent, ReflectFromWorld, ReflectMapEntities, ReflectVisitEntities,
    ReflectVisitEntitiesMut,
};
use bevy_ecs::{
    component::Component,
    entity::{Entity, VisitEntities, VisitEntitiesMut},
    traversal::Traversal,
    world::{FromWorld, World},
};
use core::{fmt::Debug, marker::PhantomData, ops::Deref};

use crate::relationship::Relationship;

use super::OneToMany;

/// Represents one half of a one-to-many relationship between an [`Entity`] and some number of other [entities](Entity).
///
/// The type of relationship is denoted by the parameter `R`.
#[derive(Component)]
#[component(
    on_insert = <Self as Relationship>::associate,
    on_replace = <Self as Relationship>::disassociate,
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
pub struct ManyToOne<R> {
    entity: Entity,
    #[cfg_attr(feature = "reflect", reflect(ignore))]
    _phantom: PhantomData<fn(&R)>,
}

impl<FK: 'static> Relationship for ManyToOne<FK> {
    type Other = OneToMany<FK>;

    fn has(&self, entity: Entity) -> bool {
        self.entity == entity
    }

    fn new(entity: Entity) -> Self {
        Self {
            entity,
            _phantom: PhantomData,
        }
    }

    fn with(self, entity: Entity) -> Self {
        Self::new(entity)
    }

    fn without(self, entity: Entity) -> Option<Self> {
        (self.entity != entity).then_some(self)
    }

    fn iter(&self) -> impl ExactSizeIterator<Item = Entity> {
        [self.entity].into_iter()
    }
}

impl<R> PartialEq for ManyToOne<R> {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity
    }
}

impl<R> Eq for ManyToOne<R> {}

impl<R> Debug for ManyToOne<R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Has One {:?} ({}) With Many ({})",
            self.entity,
            core::any::type_name::<R>(),
            core::any::type_name::<R>()
        )
    }
}

impl<R> VisitEntities for ManyToOne<R> {
    fn visit_entities<F: FnMut(Entity)>(&self, mut f: F) {
        f(self.entity);
    }
}

impl<R> VisitEntitiesMut for ManyToOne<R> {
    fn visit_entities_mut<F: FnMut(&mut Entity)>(&mut self, mut f: F) {
        f(&mut self.entity);
    }
}

// TODO: We need to impl either FromWorld or Default so OneToOne<R> can be registered as Reflect.
// This is because Reflect deserialize by creating an instance and apply a patch on top.
// However OneToOne<R> should only ever be set with a real user-defined entity. It's worth looking into
// better ways to handle cases like this.
impl<R> FromWorld for ManyToOne<R> {
    #[inline(always)]
    fn from_world(_world: &mut World) -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
            _phantom: PhantomData,
        }
    }
}

impl<R> Deref for ManyToOne<R> {
    type Target = Entity;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.entity
    }
}

/// This provides generalized hierarchy traversal for use in [event propagation].
///
/// `ManyToOne::traverse` will never form loops in properly-constructed hierarchies.
///
/// [event propagation]: bevy_ecs::observer::Trigger::propagate
impl<R: 'static> Traversal for &ManyToOne<R> {
    fn traverse(item: Self::Item<'_>) -> Option<Entity> {
        Some(item.entity)
    }
}

impl<R> ManyToOne<R> {
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
