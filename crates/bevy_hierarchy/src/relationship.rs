use bevy_ecs::{
    component::{Component, ComponentId},
    entity::Entity,
    event::{Event, Events},
    world::{DeferredWorld, World},
};
use core::marker::PhantomData;
use smallvec::SmallVec;

/// Trait representing a relationship [`Component`].
///
/// A relationship consists of two [entities](Entity), one with this [`Component`],
/// and the other with the [`Other`](Relationship::Other).
/// These entities are referred to as `primary` and `foreign` to align with typical
/// relational database terminology.
/// The `primary` owns a component which contains some number of `foreign` entities.
/// This trait is designed to ensure that those `foreign` entities also own a component of type
/// [Other](Relationship::Other), where its `foreign` entities include the aforementioned `primary`.
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

    fn associate(mut world: DeferredWorld<'_>, primary_id: Entity, _component: ComponentId) {
        world.commands().queue(move |world: &mut World| {
            let foreign_ids_len = world
                .get_entity(primary_id)
                .ok()
                .and_then(|a| a.get::<Self>())
                .map(Self::len);

            let Some(foreign_ids_len) = foreign_ids_len else {
                return;
            };

            for foreign_id_index in 0..foreign_ids_len {
                let foreign = world
                    .get_entity(primary_id)
                    .ok()
                    .and_then(|primary| primary.get::<Self>())
                    .map(|primary_relationship| primary_relationship.get(foreign_id_index).unwrap())
                    .and_then(|foreign_id| world.get_entity_mut(foreign_id).ok());

                let Some(mut foreign) = foreign else { return };

                let foreign_id = foreign.id();

                let foreign_points_to_primary = foreign
                    .get::<Self::Other>()
                    .is_some_and(|foreign_relationship| foreign_relationship.has(primary_id));

                if !foreign_points_to_primary {
                    let other = foreign
                        .take::<Self::Other>()
                        .unwrap_or(Self::Other::new(primary_id))
                        .with(primary_id);

                    foreign.insert(other);

                    if let Some(mut events) =
                        world.get_resource_mut::<Events<RelationshipEvent<Self>>>()
                    {
                        events.send(RelationshipEvent::<Self>::added(primary_id, foreign_id));
                    }
                }
            }
        });
    }

    fn disassociate(mut world: DeferredWorld<'_>, primary_id: Entity, _component: ComponentId) {
        let Some(primary_relationship) = world.get::<Self>(primary_id) else {
            unreachable!("component hook should only be called when component is available");
        };

        // Cloning to allow a user to `take` the component for modification
        // [Entity; 7] chosen to keep b_ids at 64 bytes on 64 bit platforms.
        let foreign_ids = primary_relationship
            .iter()
            .collect::<SmallVec<[Entity; 7]>>();

        world.commands().queue(move |world: &mut World| {
            for foreign_id in foreign_ids {
                let primary_points_to_foreign = world
                    .get_entity(primary_id)
                    .ok()
                    .and_then(|primary| primary.get::<Self>())
                    .is_some_and(|primary_relationship| primary_relationship.has(foreign_id));

                let foreign_points_to_primary = world
                    .get_entity(foreign_id)
                    .ok()
                    .and_then(|foreign| foreign.get::<Self::Other>())
                    .is_some_and(|foreign_relationship| foreign_relationship.has(primary_id));

                if foreign_points_to_primary && !primary_points_to_foreign {
                    if let Ok(mut foreign) = world.get_entity_mut(foreign_id) {
                        // Using a placeholder relationship to avoid triggering on_remove and on_insert
                        // hooks erroneously.
                        let mut placeholder = Self::Other::new(Entity::PLACEHOLDER);
                        let mut other = foreign.get_mut::<Self::Other>().unwrap();
                        let other = other.as_mut();

                        core::mem::swap(&mut placeholder, other);

                        if let Some(mut new_other) = placeholder.without(primary_id) {
                            core::mem::swap(&mut new_other, other);
                        } else {
                            foreign.remove::<Self::Other>();
                        }
                    }

                    if let Some(mut events) =
                        world.get_resource_mut::<Events<RelationshipEvent<Self>>>()
                    {
                        events.send(RelationshipEvent::<Self>::removed(primary_id, foreign_id));
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
