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
/// Implementing [`Trigger`] is "advanced-level" territory, and is generally unnecessary unless you are developing highly specialized
/// [`Event`] trigger logic.
///
/// Bevy comes with a number of built-in [`Trigger`] implementations (see their documentation for more info):
/// - [`GlobalTrigger`]: The [`Event`] derive defaults to using this
/// - [`EntityTrigger`]: The [`EntityEvent`] derive defaults to using this
/// - [`PropagateEntityTrigger`]: The [`EntityEvent`] derive uses this when propagation is enabled.
/// - [`EntityComponentsTrigger`]: Used by Bevy's [component lifecycle events](crate::lifecycle).
///
/// # Safety
///
/// Implementing this properly is _advanced_ soundness territory! Implementers must abide by the following:
///
/// - The `E`' [`Event::Trigger`] must be constrained to the implemented [`Trigger`] type, as part of the implementation.
///   This prevents other [`Trigger`] implementations from directly deferring to your implementation, which is a very easy
///   soundness misstep, as most [`Trigger`] implementations will invoke observers that are developed _for their specific [`Trigger`] type_.
///   Without this constraint, something like [`GlobalTrigger`] could be called for _any_ [`Event`] type, even one that expects a different
///   [`Trigger`] type. This would result in an unsound cast of [`GlobalTrigger`] reference.
///   This is not expressed as an explicit type constraint,, as the `for<'a> Event::Trigger<'a>` lifetime can mismatch explicit lifetimes in
///   some impls.
/// - Read, understand, and abide by the lifetime constraints defined in the section below:
///
/// To understand why this must be unsafe, we must think carefully about the lifetimes involved.
///
/// The core challenge is that the lifetime of the `&mut E::Trigger<'_>` that we want to create
/// within the [`ObserverRunner`] (which calls the [`Observer`](crate::observer::Observer)) may not be the same as the lifetime provided
/// by [`Event::Trigger<'a>`].
///
/// If the lifetimes are the same, then we can safely create a `&mut E::Trigger<'_>` from the [`PtrMut`]
/// passed to the observer runner function, and pass that to the observer system inside of ['On'](crate::observer::On).
///
/// If the lifetimes are not the same, then we must be careful to ensure that the `&mut E::Trigger<'_>` we create
/// does not outlive the [`PtrMut`] that was passed to the observer runner function.
/// Failing to do so could lead to use-after-free bugs.
///
/// This problem leaks into [`Trigger::trigger`] because the [`ObserverRunner`] function
/// is called from within [`Trigger::trigger`], for most real-world [`Trigger`] implementations.
///
/// This is complex, and ways to simplify this would be welcome in the future!
/// The safety requirements of this trait were prompted by this comment thread:
/// <https://github.com/bevyengine/bevy/pull/20731#discussion_r2311907935>
///
/// which also discusses some alternative designs that were considered.
///
/// [`ObserverRunner`]: crate::observer::ObserverRunner
pub unsafe trait Trigger<E: Event> {
    /// Trigger the given `event`, running every [`Observer`](crate::observer::Observer) that matches the `event`, as defined by this
    /// [`Trigger`] and the state stored on `self`.
    ///
    /// # Safety
    /// - The [`CachedObservers`] `observers` must come from the [`DeferredWorld`] `world`
    /// - [`TriggerContext`] must contain an [`EventKey`](crate::event::EventKey) that matches the `E` [`Event`] type
    /// - `observers` must correspond to observers compatible with the event type `E`
    /// - Read and abide by the "Safety" section defined in the top-level [`Trigger`] docs. Calling this function is
    ///   unintuitively risky. _Do not use it directly unless you know what you are doing_. Importantly, this should only
    ///   be called for an `event` whose [`Event::Trigger`] matches this trigger.
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

// SAFETY:
// - `E`'s [`Event::Trigger`] is contrained to [`GlobalTrigger`]
// - The implementation abides by the other safety constraints defined in [`Trigger`]
unsafe impl<E: for<'a> Event<Trigger<'a> = Self>> Trigger<E> for GlobalTrigger {
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
    /// # Safety
    /// - `observers` must come from the `world` [`DeferredWorld`], and correspond to observers that match the `event` type
    /// - `event` must point to an [`Event`]
    /// -  The `event` [`Event::Trigger`] must be [`GlobalTrigger`]
    /// - `trigger_context`'s [`TriggerContext::event_key`] must correspond to the `event` type.
    /// - Read, understand, and abide by the [`Trigger`] safety documentation
    unsafe fn trigger_internal(
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
            // SAFETY:
            // - `observers` come from `world` and match the `event` type, enforced by the call to `trigger_internal`
            // - the passed in event pointer is an `Event`, enforced by the call to `trigger_internal`
            // - `trigger` is a matching trigger type, as it comes from `self`, which is the Trigger for `event`, enforced by `trigger_internal`
            // - `trigger_context`'s event_key matches `E`, enforced by the call to `trigger_internal`
            // - this abides by the nuances defined in the `Trigger` safety docs
            unsafe {
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

// SAFETY:
// - `E`'s [`Event::Trigger`] is contrained to [`EntityTrigger`]
// - The implementation abides by the other safety constraints defined in [`Trigger`]
unsafe impl<E: EntityEvent + for<'a> Event<Trigger<'a> = Self>> Trigger<E> for EntityTrigger {
    unsafe fn trigger(
        &mut self,
        world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        event: &mut E,
    ) {
        let entity = event.event_target();
        // SAFETY:
        // - `observers` come from `world` and match the event type `E`, enforced by the call to `trigger`
        // - the passed in event pointer comes from `event`, which is an `Event`
        // - `trigger` is a matching trigger type, as it comes from `self`, which is the Trigger for `E`
        // - `trigger_context`'s event_key matches `E`, enforced by the call to `trigger`
        // - this abides by the nuances defined in the `Trigger` safety docs
        unsafe {
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
}

/// Trigger observers watching for the given entity event.
/// The `target_entity` should match the [`EntityEvent::event_target`] on `event` for logical correctness.
///
/// # Safety
/// - `observers` must come from the `world` [`DeferredWorld`], and correspond to observers that match the `event` type
/// - `event` must point to an [`Event`]
/// - `trigger` must correspond to the [`Event::Trigger`] type expected by the `event`
/// - `trigger_context`'s [`TriggerContext::event_key`] must correspond to the `event` type.
/// - Read, understand, and abide by the [`Trigger`] safety documentation
// Note: this is not an EntityTrigger method because we want to reuse this logic for the entity propagation trigger
#[inline(never)]
pub unsafe fn trigger_entity_internal(
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
        // SAFETY:
        // - `observers` come from `world` and match the `event` type, enforced by the call to `trigger_entity_internal`
        // - the passed in event pointer is an `Event`, enforced by the call to `trigger_entity_internal`
        // - `trigger` is a matching trigger type, enforced by the call to `trigger_entity_internal`
        // - `trigger_context`'s event_key matches `E`, enforced by the call to `trigger_entity_internal`
        // - this abides by the nuances defined in the `Trigger` safety docs
        unsafe {
            (runner)(
                world.reborrow(),
                *observer,
                trigger_context,
                event.reborrow(),
                trigger.reborrow(),
            );
        }
    }

    if let Some(map) = observers.entity_observers().get(&target_entity) {
        for (observer, runner) in map {
            // SAFETY:
            // - `observers` come from `world` and match the `event` type, enforced by the call to `trigger_entity_internal`
            // - the passed in event pointer is an `Event`, enforced by the call to `trigger_entity_internal`
            // - `trigger` is a matching trigger type, enforced by the call to `trigger_entity_internal`
            // - `trigger_context`'s event_key matches `E`, enforced by the call to `trigger_entity_internal`
            // - this abides by the nuances defined in the `Trigger` safety docs
            unsafe {
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

// SAFETY:
// - `E`'s [`Event::Trigger`] is contrained to [`PropagateEntityTrigger<E>`]
// - The implementation abides by the other safety constraints defined in [`Trigger`]
unsafe impl<
        const AUTO_PROPAGATE: bool,
        E: EntityEvent + for<'a> Event<Trigger<'a> = Self>,
        T: Traversal<E>,
    > Trigger<E> for PropagateEntityTrigger<AUTO_PROPAGATE, E, T>
{
    unsafe fn trigger(
        &mut self,
        mut world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        event: &mut E,
    ) {
        let mut current_entity = event.event_target();
        self.original_event_target = current_entity;
        // SAFETY:
        // - `observers` come from `world` and match the event type `E`, enforced by the call to `trigger`
        // - the passed in event pointer comes from `event`, which is an `Event`
        // - `trigger` is a matching trigger type, as it comes from `self`, which is the Trigger for `E`
        // - `trigger_context`'s event_key matches `E`, enforced by the call to `trigger`
        // - this abides by the nuances defined in the `Trigger` safety docs
        unsafe {
            trigger_entity_internal(
                world.reborrow(),
                observers,
                event.into(),
                self.into(),
                current_entity,
                trigger_context,
            );
        }

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
            // SAFETY:
            // - `observers` come from `world` and match the event type `E`, enforced by the call to `trigger`
            // - the passed in event pointer comes from `event`, which is an `Event`
            // - `trigger` is a matching trigger type, as it comes from `self`, which is the Trigger for `E`
            // - `trigger_context`'s event_key matches `E`, enforced by the call to `trigger`
            // - this abides by the nuances defined in the `Trigger` safety docs
            unsafe {
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

// SAFETY:
// - `E`'s [`Event::Trigger`] is contrained to [`EntityComponentsTrigger`]
// - The implementation abides by the other safety constraints defined in [`Trigger`]
unsafe impl<'a, E: EntityEvent + Event<Trigger<'a> = EntityComponentsTrigger<'a>>> Trigger<E>
    for EntityComponentsTrigger<'a>
{
    unsafe fn trigger(
        &mut self,
        world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        event: &mut E,
    ) {
        let entity = event.event_target();
        // SAFETY:
        // - `observers` come from `world` and match the event type `E`, enforced by the call to `trigger`
        // - the passed in event pointer comes from `event`, which is an `Event`
        // - `trigger_context`'s event_key matches `E`, enforced by the call to `trigger`
        // - this abides by the nuances defined in the `Trigger` safety docs
        unsafe {
            self.trigger_internal(world, observers, event.into(), entity, trigger_context);
        }
    }
}

impl<'a> EntityComponentsTrigger<'a> {
    /// # Safety
    /// - `observers` must come from the `world` [`DeferredWorld`]
    /// - `event` must point to an [`Event`] whose [`Event::Trigger`] is [`EntityComponentsTrigger`]
    /// - `trigger_context`'s [`TriggerContext::event_key`] must correspond to the `event` type.
    /// - Read, understand, and abide by the [`Trigger`] safety documentation
    #[inline(never)]
    unsafe fn trigger_internal(
        &mut self,
        mut world: DeferredWorld,
        observers: &CachedObservers,
        mut event: PtrMut,
        entity: Entity,
        trigger_context: &TriggerContext,
    ) {
        // SAFETY:
        // - `observers` come from `world` and match the event type `E`, enforced by the call to `trigger`
        // - the passed in event pointer comes from `event`, which is an `Event`
        // - `trigger` is a matching trigger type, as it comes from `self`, which is the Trigger for `E`
        // - `trigger_context`'s event_key matches `E`, enforced by the call to `trigger`
        // - this abides by the nuances defined in the `Trigger` safety docs
        unsafe {
            trigger_entity_internal(
                world.reborrow(),
                observers,
                event.reborrow(),
                self.into(),
                entity,
                trigger_context,
            );
        }

        // Trigger observers watching for a specific component
        for id in self.components {
            if let Some(component_observers) = observers.component_observers().get(id) {
                for (observer, runner) in component_observers.global_observers() {
                    // SAFETY:
                    // - `observers` come from `world` and match the `event` type, enforced by the call to `trigger_internal`
                    // - the passed in event pointer is an `Event`, enforced by the call to `trigger_internal`
                    // - `trigger` is a matching trigger type, enforced by the call to `trigger_internal`
                    // - `trigger_context`'s event_key matches `E`, enforced by the call to `trigger_internal`
                    // - this abides by the nuances defined in the `Trigger` safety docs
                    unsafe {
                        (runner)(
                            world.reborrow(),
                            *observer,
                            trigger_context,
                            event.reborrow(),
                            self.into(),
                        );
                    }
                }

                if let Some(map) = component_observers
                    .entity_component_observers()
                    .get(&entity)
                {
                    for (observer, runner) in map {
                        // SAFETY:
                        // - `observers` come from `world` and match the `event` type, enforced by the call to `trigger_internal`
                        // - the passed in event pointer is an `Event`, enforced by the call to `trigger_internal`
                        // - `trigger` is a matching trigger type, enforced by the call to `trigger_internal`
                        // - `trigger_context`'s event_key matches `E`, enforced by the call to `trigger_internal`
                        // - this abides by the nuances defined in the `Trigger` safety docs
                        unsafe {
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
}
