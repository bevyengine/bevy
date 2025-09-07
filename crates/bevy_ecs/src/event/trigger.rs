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

/// [`Trigger`] determines _how_ an [`Event`] is triggered when [`World::trigger`](crate::world::World::trigger) is called.
/// This decides which [`Observer`](crate::observer::Observer)s will run, what data gets passed to them, and the order they will
/// be executed in.
///
/// Implementing [`Trigger`] is "advanced-level" terrority, and is generally unnecessary unless you are developing highly specialized
/// [`Event`] trigger logic.
///
/// Bevy comes with a number of built-in [`Trigger`] implementations (see their documentation for more info):
/// - [`GlobalTrigger`]: The [`Event`] derive defaults to using this
/// - [`EntityTrigger`]: The [`EntityEvent`] derive defaults to using this
/// - [`PropagateEntityTrigger`]: The [`EntityEvent`] derive uses this when propagation is enabled.
/// - [`EntityComponentsTrigger`]: Used by Bevy's [component lifecycle events](crate::lifecycle).
///
/// SAFETY: TODO!
pub unsafe trait Trigger<E: Event> {
    /// Trigger the given `event`, running every [`Observer`](crate::observer::Observer) that matches the `event`, as defined by this
    /// [`Trigger`] and the state stored on `self`.
    ///
    /// SAFETY: TODO!
    // To understand why this must be unsafe, we must think carefully about the lifetimes involved.
    //
    // The core challenge is that the lifetime of the `&mut E::Trigger<'_>` that we want to create
    // within our `observer_system_runner` may not be the same as the lifetime provided by Event::Trigger<'a>.
    //
    // If the lifetimes are the same, then we can safely create a `&mut E::Trigger<'_>` from the `PtrMut`
    // passed to the observer runner function, and pass that to the observer system inside of 'On'.
    //
    // If the lifetimes are not the same, then we must be careful to ensure that the `&mut E::Trigger<'_>` we create
    // does not outlive the `PtrMut` that was passed to the observer runner function.
    // Failing to do so could lead to use-after-free bugs.
    //
    // To avoid this, we require that the caller of this function (i.e. the code that triggers the event)
    // ensures that TODO.
    //
    // This is complex, and ways to simplify this would be welcome in the future!
    // The safety requirements of this method were prompted by this comment thread:
    // https://github.com/bevyengine/bevy/pull/20731#discussion_r2311907935
    //
    // which also discusses some alternative designs that were considered.
    unsafe fn trigger(
        &mut self,
        world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        event: &mut E,
    );
}

/// A [`Trigger`] that runs _every_ "global" [`Observer`](crate::observer::Observer) (ex: registered via [`World::add_observer`](crate::world::World::add_observer))
/// that matches the given [`Event`].
///
/// The [`Event`] derive defaults to using this [`Trigger`], and it is usable for any [`Event`] type.
#[derive(Default)]
pub struct GlobalTrigger;

unsafe impl<E: Event> Trigger<E> for GlobalTrigger {
    /// SAFETY: TODO!
    unsafe fn trigger(
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

/// An [`EntityEvent`] [`Trigger`] that does two things:
/// - Runs all "global" [`Observer`] (ex: registered via [`World::add_observer`](crate::world::World::add_observer))
///   that matches the given [`Event`]. This is the same behavior as [`GlobalTrigger`].
/// - Runs every "entity scoped" [`Observer`] that watches the given [`EntityEvent::event_target`] entity.
///
/// The [`EntityEvent`] derive defaults to using this [`Trigger`], and it is usable for any [`EntityEvent`] type.
///
/// [`Observer`]: crate::observer::Observer
#[derive(Default)]
pub struct EntityTrigger;

unsafe impl<E: EntityEvent> Trigger<E> for EntityTrigger {
    /// SAFETY: TODO!
    unsafe fn trigger(
        &mut self,
        world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        event: &mut E,
    ) {
        let entity = event.event_target();
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

/// Trigger observers watching for the given entity event.
/// The `target_entity` should match the [`EntityEvent::event_target`] on `event` for logical correctness.
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

/// An [`EntityEvent`] [`Trigger`] that behaves like [`EntityTrigger`], but "propagates" the event
/// using an [`Entity`] [`Traversal`]. At each step in the propagation, the [`EntityTrigger`] logic will
/// be run, until [`PropagateEntityTrigger::propagate`] is false, or there are no entities left to traverse.
///
/// This is used by the [`EntityEvent`] derive when `#[entity_event(propagate)]` is enabled. It is usable by every
/// [`EntityEvent`] type.
///
/// If `AUTO_PROPAGATE` is `true`, [`PropagateEntityTrigger::propagate`] will default to `true`.
pub struct PropagateEntityTrigger<const AUTO_PROPAGATE: bool, E: EntityEvent, T: Traversal<E>> {
    /// The original [`Entity`] the [`Event`] was _first_ triggered for.
    pub original_event_target: Entity,

    /// Whether or not to continue propagating using the `T` [`Traversal`]. If this is false,
    /// The [`Traversal`] will stop on the current entity.
    pub propagate: bool,

    _marker: PhantomData<(E, T)>,
}

impl<const AUTO_PROPAGATE: bool, E: EntityEvent, T: Traversal<E>> Default
    for PropagateEntityTrigger<AUTO_PROPAGATE, E, T>
{
    fn default() -> Self {
        Self {
            original_event_target: Entity::PLACEHOLDER,
            propagate: AUTO_PROPAGATE,
            _marker: Default::default(),
        }
    }
}

unsafe impl<const AUTO_PROPAGATE: bool, E: EntityEvent, T: Traversal<E>> Trigger<E>
    for PropagateEntityTrigger<AUTO_PROPAGATE, E, T>
{
    /// SAFETY: TODO!
    unsafe fn trigger(
        &mut self,
        mut world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        event: &mut E,
    ) {
        let mut current_entity = event.event_target();
        self.original_event_target = current_entity;
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

            *event.event_target_mut() = current_entity;
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

/// An [`EntityEvent`] [`Trigger`] that, in addition to behaving like a normal [`EntityTrigger`], _also_ runs observers
/// that watch for components that match the slice of [`ComponentId`]s referenced in [`EntityComponentsTrigger`]. This includes
/// both _global_ observers of those components and "entity scoped" observers that watch the [`EntityEvent::event_target`].
///
/// This is used by Bevy's built-in [lifecycle events](crate::lifecycle).
#[derive(Default)]
pub struct EntityComponentsTrigger<'a> {
    /// All of the components whose observers were triggered together for the target entity. For example,
    /// if components `A` and `B` are added together, producing the [`Add`](crate::lifecycle::Add) event, this will
    /// contain the [`ComponentId`] for both `A` and `B`.
    pub components: &'a [ComponentId],
}

unsafe impl<'a, E: EntityEvent> Trigger<E> for EntityComponentsTrigger<'a> {
    /// SAFETY: TODO!
    unsafe fn trigger(
        &mut self,
        world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        event: &mut E,
    ) {
        let entity = event.event_target();
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

        // Trigger observers watching for a specific component
        for id in self.components {
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
