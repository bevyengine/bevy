use bevy_ecs::{
    component::{Component, ComponentId},
    entity::Entity,
    event::{Event, Events},
    world::{DeferredWorld, World},
};
use core::marker::PhantomData;
use smallvec::SmallVec;

/// Trait representing a relationship [`Component`].
/// A relationship consists of two [entities](Entity), one with this [`Component`],
/// and the other with the [`Other`](Relationship::Other).
pub(crate) trait Relationship: Component + Sized {
    /// The other [`Component`] used to form this relationship.
    type Other: Relationship<Other = Self>;

    /// Whether this [`Relationship`] [`Component`] has the provided [`Entity`].
    fn has(&self, entity: Entity) -> bool;

    /// Create a new [`Relationship`] [`Component`] with the provided [`Entity`].
    fn new(entity: Entity) -> Self;

    /// Modify an existing [`Relationship`] [`Component`] to ensure it includes
    /// the provided [`Entity`].
    fn with(self, entity: Entity) -> Self;

    /// Modify an existing [`Relationship`] [`Component`] to ensure it does not
    /// include the provided [`Entity`].
    ///
    /// Returns [`None`] if this [`Entity`] is the last member of this relationship.
    fn without(self, entity: Entity) -> Option<Self>;

    /// Iterate over all [entities](Entity) this [`Relationship`] [`Component`] contains.
    fn iter(&self) -> impl ExactSizeIterator<Item = Entity>;

    fn len(&self) -> usize {
        self.iter().len()
    }

    fn get(&self, index: usize) -> Option<Entity> {
        self.iter().nth(index)
    }

    fn associate(mut world: DeferredWorld<'_>, a_id: Entity, _component: ComponentId) {
        world.commands().queue(move |world: &mut World| {
            let b_ids_len = world
                .get_entity(a_id)
                .ok()
                .and_then(|a| a.get::<Self>())
                .map(Self::len);

            let Some(b_ids_len) = b_ids_len else { return };

            for b_id_index in 0..b_ids_len {
                let b = world
                    .get_entity(a_id)
                    .ok()
                    .and_then(|a| a.get::<Self>())
                    .map(|a_relationship| a_relationship.get(b_id_index).unwrap())
                    .and_then(|b_id| world.get_entity_mut(b_id).ok());

                let Some(mut b) = b else { return };

                let _b_id = b.id();

                let b_points_to_a = b
                    .get::<Self::Other>()
                    .is_some_and(|b_relationship| b_relationship.has(a_id));

                if !b_points_to_a {
                    let other = b
                        .take::<Self::Other>()
                        .unwrap_or(Self::Other::new(a_id))
                        .with(a_id);

                    b.insert(other);

                    if let Some(mut events) =
                        world.get_resource_mut::<Events<RelationshipEvent<Self>>>()
                    {
                        events.send(RelationshipEvent::<Self>::added(a_id, _b_id));
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
        // [Entity; 7] chosen to keep b_ids at 64 bytes on 64 bit platforms.
        let b_ids = a_relationship.iter().collect::<SmallVec<[Entity; 7]>>();

        world.commands().queue(move |world: &mut World| {
            for b_id in b_ids {
                let a_points_to_b = world
                    .get_entity(a_id)
                    .ok()
                    .and_then(|a| a.get::<Self>())
                    .is_some_and(|a_relationship| a_relationship.has(b_id));

                let b_points_to_a = world
                    .get_entity(b_id)
                    .ok()
                    .and_then(|b| b.get::<Self::Other>())
                    .is_some_and(|b_relationship| b_relationship.has(a_id));

                if b_points_to_a && !a_points_to_b {
                    if let Ok(mut b) = world.get_entity_mut(b_id) {
                        // Using a placeholder relationship to avoid triggering on_remove and on_insert
                        // hooks erroneously.
                        let mut placeholder = Self::Other::new(Entity::PLACEHOLDER);
                        let mut other = b.get_mut::<Self::Other>().unwrap();
                        let other = other.as_mut();

                        core::mem::swap(&mut placeholder, other);

                        if let Some(mut new_other) = placeholder.without(a_id) {
                            core::mem::swap(&mut new_other, other);
                        } else {
                            b.remove::<Self::Other>();
                        }
                    }

                    if let Some(mut events) =
                        world.get_resource_mut::<Events<RelationshipEvent<Self>>>()
                    {
                        events.send(RelationshipEvent::<Self>::removed(a_id, b_id));
                    }
                }
            }
        });
    }
}

/// A relationship event.
#[derive(Event)]
pub enum RelationshipEvent<R> {
    /// A relationship was added between two [entities](Entity).
    Added(RelationshipEventDetails<R>),
    /// A relationship was removed from two [entities](Entity).
    Removed(RelationshipEventDetails<R>),
}

impl<R> RelationshipEvent<R> {
    /// Create a new [`Added`](RelationshipEvent::Added) [`Event`]
    pub const fn added(primary: Entity, foreign: Entity) -> Self {
        Self::Added(RelationshipEventDetails::new(primary, foreign))
    }

    /// Create a new [`Removed`](RelationshipEvent::Removed) [`Event`]
    pub const fn removed(primary: Entity, foreign: Entity) -> Self {
        Self::Removed(RelationshipEventDetails::new(primary, foreign))
    }

    /// Get the primary [`Entity`] in this [`Event`].
    /// The primary is the _cause_ of the event, while the foreign is the relation.
    pub const fn primary(&self) -> Entity {
        match self {
            Self::Added(details) | Self::Removed(details) => details.primary(),
        }
    }

    /// Get the foreign [`Entity`] in this [`Event`].
    /// The primary is the _cause_ of the event, while the foreign is the relation.
    pub const fn foreign(&self) -> Entity {
        match self {
            Self::Added(details) | Self::Removed(details) => details.foreign(),
        }
    }
}

impl<R> core::fmt::Debug for RelationshipEvent<R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Added(arg0) => f.debug_tuple("Added").field(arg0).finish(),
            Self::Removed(arg0) => f.debug_tuple("Removed").field(arg0).finish(),
        }
    }
}

impl<R> PartialEq for RelationshipEvent<R> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Added(l0), Self::Added(r0)) | (Self::Removed(l0), Self::Removed(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl<R> Eq for RelationshipEvent<R> {}

impl<R> Clone for RelationshipEvent<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R> Copy for RelationshipEvent<R> {}

/// The details of a [`RelationshipEvent`].
pub struct RelationshipEventDetails<R> {
    primary: Entity,
    foreign: Entity,
    phantom_data: PhantomData<fn(R)>,
}

impl<R> RelationshipEventDetails<R> {
    /// Create a new [`RelationshipEventDetails`] for a `primary` and a `foreign` [`Entity`].
    /// The `primary` [`Entity`] is the cause of the [`Event`], while the `foreign`
    /// is the other member of the relationship.
    pub const fn new(primary: Entity, foreign: Entity) -> Self {
        Self {
            primary,
            foreign,
            phantom_data: PhantomData,
        }
    }

    /// Get the [`Entity`] that caused this [`Event`] to be triggered.
    pub const fn primary(&self) -> Entity {
        self.primary
    }

    /// Get the [`Entity`] related to the `primary` [`Entity`].
    pub const fn foreign(&self) -> Entity {
        self.foreign
    }
}

impl<R> core::fmt::Debug for RelationshipEventDetails<R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RelationshipEventDetails")
            .field("primary", &self.primary)
            .field("foreign", &self.foreign)
            .finish()
    }
}

impl<R> PartialEq for RelationshipEventDetails<R> {
    fn eq(&self, other: &Self) -> bool {
        self.primary == other.primary && self.foreign == other.foreign
    }
}

impl<R> Eq for RelationshipEventDetails<R> {}

impl<R> Clone for RelationshipEventDetails<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R> Copy for RelationshipEventDetails<R> {}
