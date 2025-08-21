use crate::{
    component::ComponentId,
    entity::Entity,
    event::Event,
    observer::{CachedObservers, TriggerContext},
    traversal::Traversal,
    world::DeferredWorld,
};
use bevy_ptr::{Ptr, PtrMut};
use core::marker::PhantomData;

pub trait Trigger: Default {
    type Target<'a>;
    fn trigger(
        &mut self,
        world: DeferredWorld,
        observers: &CachedObservers,
        event: PtrMut,
        target: &Self::Target<'_>,
        trigger_context: &TriggerContext,
    );
}

#[derive(Default)]
pub struct GlobalTrigger;

impl Trigger for GlobalTrigger {
    type Target<'a> = ();

    fn trigger(
        &mut self,
        mut world: DeferredWorld,
        observers: &CachedObservers,
        mut event: PtrMut,
        target: &Self::Target<'_>,
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
                target.into(),
                self.into(),
            );
        }
    }
}

#[derive(Default)]
pub struct EntityTrigger;

impl Trigger for EntityTrigger {
    type Target<'a> = Entity;
    fn trigger(
        &mut self,
        world: DeferredWorld,
        observers: &CachedObservers,
        event: PtrMut,
        target: &Self::Target<'_>,
        trigger_context: &TriggerContext,
    ) {
        trigger_entity_raw(
            world,
            observers,
            event,
            target.into(),
            target,
            self.into(),
            trigger_context,
        );
    }
}

fn trigger_entity_raw(
    mut world: DeferredWorld,
    observers: &CachedObservers,
    mut event: PtrMut,
    target: Ptr,
    target_entity: &Entity,
    mut trigger: PtrMut,
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
            target,
            trigger.reborrow(),
        );
    }

    if let Some(map) = observers.entity_observers().get(target_entity) {
        for (observer, runner) in map {
            (runner)(
                world.reborrow(),
                *observer,
                trigger_context,
                event.reborrow(),
                target,
                trigger.reborrow(),
            );
        }
    }
}

pub struct PropagateEntityTrigger<
    const AUTO_PROPAGATE: bool,
    E: for<'t> Event<Target<'t> = Entity>,
    T: Traversal<E>,
> {
    pub original_entity: Entity,
    pub propagate: bool,
    _marker: PhantomData<(E, T)>,
}

impl<const AUTO_PROPAGATE: bool, E: for<'t> Event<Target<'t> = Entity>, T: Traversal<E>> Default
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

impl<const AUTO_PROPAGATE: bool, E: for<'t> Event<Target<'t> = Entity>, T: Traversal<E>> Trigger
    for PropagateEntityTrigger<AUTO_PROPAGATE, E, T>
{
    type Target<'a> = Entity;

    fn trigger(
        &mut self,
        mut world: DeferredWorld,
        observers: &CachedObservers,
        mut event: PtrMut,
        target: &Self::Target<'_>,
        trigger_context: &TriggerContext,
    ) {
        self.original_entity = *target;
        trigger_entity_raw(
            world.reborrow(),
            observers,
            event.reborrow(),
            target.into(),
            target,
            self.into(),
            trigger_context,
        );

        let mut current_target = *target;
        loop {
            if !self.propagate {
                return;
            }
            if let Ok(entity) = world.get_entity(current_target)
                && let Some(item) = entity.get_components::<T>()
                && let Some(traverse_to) =
                    // TODO: Sort out the safety of this
                    T::traverse(item, unsafe { event.reborrow().deref_mut() })
            {
                current_target = traverse_to;
            } else {
                break;
            }

            trigger_entity_raw(
                world.reborrow(),
                observers,
                event.reborrow(),
                (&current_target).into(),
                &current_target,
                self.into(),
                trigger_context,
            );
        }
    }
}

#[derive(Default)]
pub struct EntityComponentsTrigger;

pub struct EntityComponents<'a> {
    pub entity: Entity,
    pub components: &'a [ComponentId],
}

impl Trigger for EntityComponentsTrigger {
    type Target<'a> = EntityComponents<'a>;

    fn trigger(
        &mut self,
        mut world: DeferredWorld,
        observers: &CachedObservers,
        mut event: PtrMut,
        target: &Self::Target<'_>,
        trigger_context: &TriggerContext,
    ) {
        trigger_entity_raw(
            world.reborrow(),
            observers,
            event.reborrow(),
            target.into(),
            &target.entity,
            self.into(),
            trigger_context,
        );

        // Trigger observers listening to this trigger targeting a specific component
        for id in target.components {
            if let Some(component_observers) = observers.component_observers().get(id) {
                for (observer, runner) in component_observers.global_observers() {
                    (runner)(
                        world.reborrow(),
                        *observer,
                        trigger_context,
                        event.reborrow(),
                        target.into(),
                        self.into(),
                    );
                }

                if let Some(map) = component_observers
                    .entity_component_observers()
                    .get(&target.entity)
                {
                    for (observer, runner) in map {
                        (runner)(
                            world.reborrow(),
                            *observer,
                            trigger_context,
                            event.reborrow(),
                            target.into(),
                            self.into(),
                        );
                    }
                }
            }
        }
    }
}

pub trait EntityTarget {
    fn entity(&self) -> Entity;
}

impl EntityTarget for Entity {
    fn entity(&self) -> Entity {
        *self
    }
}

impl<'a> EntityTarget for EntityComponents<'a> {
    fn entity(&self) -> Entity {
        self.entity
    }
}
