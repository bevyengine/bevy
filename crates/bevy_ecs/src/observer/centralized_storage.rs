//! Centralized storage for observers, allowing for efficient look-ups.
//!
//! This has multiple levels:
//! - [`World::observers`] provides access to [`Observers`], which is a central storage for all observers.
//! - [`Observers`] contains multiple distinct caches in the form of [`CachedObservers`].
//!     - Most observers are looked up by the [`ComponentId`] of the event they are observing
//!     - Lifecycle observers have their own fields to save lookups.
//! - [`CachedObservers`] contains maps of [`ObserverRunner`]s, which are the actual functions that will be run when the observer is triggered.
//!     - These are split by target type, in order to allow for different lookup strategies.
//!     - [`CachedComponentObservers`] is one of these maps, which contains observers that are specifically targeted at a component.

use bevy_platform::collections::HashMap;

use crate::{
    archetype::ArchetypeFlags,
    change_detection::MaybeLocation,
    component::ComponentId,
    entity::EntityHashMap,
    observer::{ObserverRunner, ObserverTrigger},
    prelude::*,
    world::DeferredWorld,
};

/// An internal lookup table tracking all of the observers in the world.
///
/// Stores a cache mapping trigger ids to the registered observers.
/// Some observer kinds (like [lifecycle](crate::lifecycle) observers) have a dedicated field,
/// saving lookups for the most common triggers.
///
/// This can be accessed via [`World::observers`].
#[derive(Default, Debug)]
pub struct Observers {
    // Cached ECS observers to save a lookup most common triggers.
    add: CachedObservers,
    insert: CachedObservers,
    replace: CachedObservers,
    remove: CachedObservers,
    despawn: CachedObservers,
    // Map from trigger type to set of observers listening to that trigger
    cache: HashMap<ComponentId, CachedObservers>,
}

impl Observers {
    pub(crate) fn get_observers_mut(&mut self, event_type: ComponentId) -> &mut CachedObservers {
        use crate::lifecycle::*;

        match event_type {
            ADD => &mut self.add,
            INSERT => &mut self.insert,
            REPLACE => &mut self.replace,
            REMOVE => &mut self.remove,
            DESPAWN => &mut self.despawn,
            _ => self.cache.entry(event_type).or_default(),
        }
    }

    /// Attempts to get the observers for the given `event_type`.
    ///
    /// When accessing the observers for lifecycle events, such as [`Add`], [`Insert`], [`Replace`], [`Remove`], and [`Despawn`],
    /// use the [`ComponentId`] constants from the [`lifecycle`](crate::lifecycle) module.
    pub fn try_get_observers(&self, event_type: ComponentId) -> Option<&CachedObservers> {
        use crate::lifecycle::*;

        match event_type {
            ADD => Some(&self.add),
            INSERT => Some(&self.insert),
            REPLACE => Some(&self.replace),
            REMOVE => Some(&self.remove),
            DESPAWN => Some(&self.despawn),
            _ => self.cache.get(&event_type),
        }
    }

    /// This will run the observers of the given `event_type`, targeting the given `entity` and `components`.
    pub(crate) fn invoke<T>(
        mut world: DeferredWorld,
        event_type: ComponentId,
        current_target: Option<Entity>,
        original_target: Option<Entity>,
        components: impl Iterator<Item = ComponentId> + Clone,
        data: &mut T,
        propagate: &mut bool,
        caller: MaybeLocation,
    ) {
        // SAFETY: You cannot get a mutable reference to `observers` from `DeferredWorld`
        let (mut world, observers) = unsafe {
            let world = world.as_unsafe_world_cell();
            // SAFETY: There are no outstanding world references
            world.increment_trigger_id();
            let observers = world.observers();
            let Some(observers) = observers.try_get_observers(event_type) else {
                return;
            };
            // SAFETY: The only outstanding reference to world is `observers`
            (world.into_deferred(), observers)
        };

        let trigger_for_components = components.clone();

        let mut trigger_observer = |(&observer, runner): (&Entity, &ObserverRunner)| {
            (runner)(
                world.reborrow(),
                ObserverTrigger {
                    observer,
                    event_type,
                    components: components.clone().collect(),
                    current_target,
                    original_target,
                    caller,
                },
                data.into(),
                propagate,
            );
        };
        // Trigger observers listening for any kind of this trigger
        observers
            .global_observers
            .iter()
            .for_each(&mut trigger_observer);

        // Trigger entity observers listening for this kind of trigger
        if let Some(target_entity) = current_target {
            if let Some(map) = observers.entity_observers.get(&target_entity) {
                map.iter().for_each(&mut trigger_observer);
            }
        }

        // Trigger observers listening to this trigger targeting a specific component
        trigger_for_components.for_each(|id| {
            if let Some(component_observers) = observers.component_observers.get(&id) {
                component_observers
                    .global_observers
                    .iter()
                    .for_each(&mut trigger_observer);

                if let Some(target_entity) = current_target {
                    if let Some(map) = component_observers
                        .entity_component_observers
                        .get(&target_entity)
                    {
                        map.iter().for_each(&mut trigger_observer);
                    }
                }
            }
        });
    }

    pub(crate) fn is_archetype_cached(event_type: ComponentId) -> Option<ArchetypeFlags> {
        use crate::lifecycle::*;

        match event_type {
            ADD => Some(ArchetypeFlags::ON_ADD_OBSERVER),
            INSERT => Some(ArchetypeFlags::ON_INSERT_OBSERVER),
            REPLACE => Some(ArchetypeFlags::ON_REPLACE_OBSERVER),
            REMOVE => Some(ArchetypeFlags::ON_REMOVE_OBSERVER),
            DESPAWN => Some(ArchetypeFlags::ON_DESPAWN_OBSERVER),
            _ => None,
        }
    }

    pub(crate) fn update_archetype_flags(
        &self,
        component_id: ComponentId,
        flags: &mut ArchetypeFlags,
    ) {
        if self.add.component_observers.contains_key(&component_id) {
            flags.insert(ArchetypeFlags::ON_ADD_OBSERVER);
        }

        if self.insert.component_observers.contains_key(&component_id) {
            flags.insert(ArchetypeFlags::ON_INSERT_OBSERVER);
        }

        if self.replace.component_observers.contains_key(&component_id) {
            flags.insert(ArchetypeFlags::ON_REPLACE_OBSERVER);
        }

        if self.remove.component_observers.contains_key(&component_id) {
            flags.insert(ArchetypeFlags::ON_REMOVE_OBSERVER);
        }

        if self.despawn.component_observers.contains_key(&component_id) {
            flags.insert(ArchetypeFlags::ON_DESPAWN_OBSERVER);
        }
    }
}

/// Collection of [`ObserverRunner`] for [`Observer`] registered to a particular event.
///
/// This is stored inside of [`Observers`], specialized for each kind of observer.
#[derive(Default, Debug)]
pub struct CachedObservers {
    // Observers listening for any time this event is fired, regardless of target
    // This will also respond to events targeting specific components or entities
    pub(super) global_observers: ObserverMap,
    // Observers listening for this trigger fired at a specific component
    pub(super) component_observers: HashMap<ComponentId, CachedComponentObservers>,
    // Observers listening for this trigger fired at a specific entity
    pub(super) entity_observers: EntityHashMap<ObserverMap>,
}

impl CachedObservers {
    /// Returns the observers listening for this trigger, regardless of target.
    /// These observers will also respond to events targeting specific components or entities.
    pub fn global_observers(&self) -> &ObserverMap {
        &self.global_observers
    }

    /// Returns the observers listening for this trigger targeting components.
    pub fn get_component_observers(&self) -> &HashMap<ComponentId, CachedComponentObservers> {
        &self.component_observers
    }

    /// Returns the observers listening for this trigger targeting entities.
    pub fn entity_observers(&self) -> &HashMap<ComponentId, CachedComponentObservers> {
        &self.component_observers
    }
}

/// Map between an observer entity and its [`ObserverRunner`]
pub type ObserverMap = EntityHashMap<ObserverRunner>;

/// Collection of [`ObserverRunner`] for [`Observer`] registered to a particular event targeted at a specific component.
///
/// This is stored inside of [`CachedObservers`].
#[derive(Default, Debug)]
pub struct CachedComponentObservers {
    // Observers listening to events targeting this component, but not a specific entity
    pub(super) global_observers: ObserverMap,
    // Observers listening to events targeting this component on a specific entity
    pub(super) entity_component_observers: EntityHashMap<ObserverMap>,
}

impl CachedComponentObservers {
    /// Returns the observers listening for this trigger, regardless of target.
    /// These observers will also respond to events targeting specific entities.
    pub fn global_observers(&self) -> &ObserverMap {
        &self.global_observers
    }

    /// Returns the observers listening for this trigger targeting this component on a specific entity.
    pub fn entity_component_observers(&self) -> &EntityHashMap<ObserverMap> {
        &self.entity_component_observers
    }
}
