use crate::{
    component::ComponentId,
    entity::Entity,
    event::{EntityEvent, Event},
    observer::{CachedObservers, TriggerContext},
    traversal::Traversal,
    world::DeferredWorld,
};
use bevy_ptr::PtrMut;
use core::marker::PhantomData;

pub trait Trigger<E: Event> {
    fn trigger(
        &mut self,
        world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        event: &mut E,
    );
}

#[derive(Default)]
pub struct GlobalTrigger;

impl<E: Event> Trigger<E> for GlobalTrigger {
    fn trigger(
        &mut self,
        world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        event: &mut E,
    ) {
        self.trigger_internal(world, observers, trigger_context, event.into());
    }
}

impl GlobalTrigger {
    fn trigger_internal(
        &mut self,
        mut world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        mut event: PtrMut,
    ) {
        // SAFETY: `observers` is the only active reference to something in `world`
        unsafe {
            world.as_unsafe_world_cell().increment_trigger_id();
        }
        for (observer, runner) in observers.global_observers() {
            (runner)(
                world.reborrow(),
                *observer,
                trigger_context,
                event.reborrow(),
                self.into(),
            );
        }
    }
}

#[derive(Default)]
pub struct EntityTrigger;

impl<E: EntityEvent> Trigger<E> for EntityTrigger {
    fn trigger(
        &mut self,
        world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        event: &mut E,
    ) {
        let entity = event.entity();
        trigger_entity_internal(
            world,
            observers,
            event.into(),
            self.into(),
            entity,
            trigger_context,
        );
    }
}

/// Trigger observers listening for the given entity event.
/// The `target_entity` should match the `EntityEvent::entity` on `event` for logical correctness.
// Note: this is not an EntityTrigger method because we want to reuse this logic for the entity propagation trigger
#[inline(never)]
pub fn trigger_entity_internal(
    mut world: DeferredWorld,
    observers: &CachedObservers,
    mut event: PtrMut,
    mut trigger: PtrMut,
    target_entity: Entity,
    trigger_context: &TriggerContext,
) {
    // SAFETY: there are no outstanding world references
    unsafe {
        world.as_unsafe_world_cell().increment_trigger_id();
    }
    for (observer, runner) in observers.global_observers() {
        (runner)(
            world.reborrow(),
            *observer,
            trigger_context,
            event.reborrow(),
            trigger.reborrow(),
        );
    }

    if let Some(map) = observers.entity_observers().get(&target_entity) {
        for (observer, runner) in map {
            (runner)(
                world.reborrow(),
                *observer,
                trigger_context,
                event.reborrow(),
                trigger.reborrow(),
            );
        }
    }
}

pub struct PropagateEntityTrigger<const AUTO_PROPAGATE: bool, E: EntityEvent, T: Traversal<E>> {
    pub original_entity: Entity,
    pub propagate: bool,
    _marker: PhantomData<(E, T)>,
}

impl<const AUTO_PROPAGATE: bool, E: EntityEvent, T: Traversal<E>> Default
    for PropagateEntityTrigger<AUTO_PROPAGATE, E, T>
{
    fn default() -> Self {
        Self {
            original_entity: Entity::PLACEHOLDER,
            propagate: AUTO_PROPAGATE,
            _marker: Default::default(),
        }
    }
}

impl<const AUTO_PROPAGATE: bool, E: EntityEvent, T: Traversal<E>> Trigger<E>
    for PropagateEntityTrigger<AUTO_PROPAGATE, E, T>
{
    fn trigger(
        &mut self,
        mut world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        event: &mut E,
    ) {
        let mut current_entity = event.entity();
        self.original_entity = current_entity;
        trigger_entity_internal(
            world.reborrow(),
            observers,
            event.into(),
            self.into(),
            current_entity,
            trigger_context,
        );

        loop {
            if !self.propagate {
                return;
            }
            if let Ok(entity) = world.get_entity(current_entity)
                && let Some(item) = entity.get_components::<T>()
                && let Some(traverse_to) = T::traverse(item, event)
            {
                current_entity = traverse_to;
            } else {
                break;
            }

            *event.entity_mut() = current_entity;
            trigger_entity_internal(
                world.reborrow(),
                observers,
                event.into(),
                self.into(),
                current_entity,
                trigger_context,
            );
        }
    }
}

#[derive(Default)]
pub struct EntityComponentsTrigger<'a>(pub &'a [ComponentId]);

impl<'a, E: EntityEvent> Trigger<E> for EntityComponentsTrigger<'a> {
    fn trigger(
        &mut self,
        world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        event: &mut E,
    ) {
        let entity = event.entity();
        self.trigger_internal(world, observers, event.into(), entity, trigger_context);
    }
}

impl<'a> EntityComponentsTrigger<'a> {
    #[inline(never)]
    fn trigger_internal(
        &mut self,
        mut world: DeferredWorld,
        observers: &CachedObservers,
        mut event: PtrMut,
        entity: Entity,
        trigger_context: &TriggerContext,
    ) {
        trigger_entity_internal(
            world.reborrow(),
            observers,
            event.reborrow(),
            self.into(),
            entity,
            trigger_context,
        );

        // Trigger observers listening to this trigger targeting a specific component
        for id in self.0 {
            if let Some(component_observers) = observers.component_observers().get(id) {
                for (observer, runner) in component_observers.global_observers() {
                    (runner)(
                        world.reborrow(),
                        *observer,
                        trigger_context,
                        event.reborrow(),
                        self.into(),
                    );
                }

                if let Some(map) = component_observers
                    .entity_component_observers()
                    .get(&entity)
                {
                    for (observer, runner) in map {
                        (runner)(
                            world.reborrow(),
                            *observer,
                            trigger_context,
                            event.reborrow(),
                            self.into(),
                        );
                    }
                }
            }
        }
    }
}
