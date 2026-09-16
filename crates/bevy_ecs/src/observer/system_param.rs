//! System parameters for working with observers.

use crate::{
    change_detection::MaybeLocation,
    event::{Event, EventKey, EventPattern, EventPatternTrigger, PropagateEntityTrigger},
    prelude::*,
    traversal::Traversal,
};
use bevy_ptr::Ptr;
use core::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

/// A [system parameter] used by an observer to process events. See [`Observer`] and [`Event`] for examples.
///
/// `On` contains the triggered [`Event`] data for a given run of an `Observer`. It also provides access to the
/// [`Trigger`](crate::event::Trigger), which for things like [`EntityEvent`] with a [`PropagateEntityTrigger`],
/// includes control over event propagation.
///
/// [system parameter]: crate::system::SystemParam
// SAFETY WARNING!
// this type must _never_ expose anything with the 'w lifetime
// See the safety discussion on `Trigger` for more details.
pub struct On<'w, 't, E: EventPattern> {
    observer: Entity,
    // SAFETY WARNING: never expose this 'w lifetime
    event: &'w mut E::Event,
    // SAFETY WARNING: never expose this 'w lifetime
    trigger: &'w mut EventPatternTrigger<'t, E>,
    // SAFETY WARNING: never expose this 'w lifetime
    trigger_context: &'w TriggerContext,
}

impl<'w, 't, E: EventPattern> On<'w, 't, E> {
    /// Creates a new instance of [`On`] for the given triggered event.
    pub fn new(
        event: &'w mut E::Event,
        observer: Entity,
        trigger: &'w mut EventPatternTrigger<'t, E>,
        trigger_context: &'w TriggerContext,
    ) -> Self {
        Self {
            event,
            observer,
            trigger,
            trigger_context,
        }
    }

    /// Returns the event type of this [`On`] instance.
    pub fn event_key(&self) -> EventKey {
        self.trigger_context.event_key
    }

    /// Returns a reference to the triggered event.
    pub fn event(&self) -> &E::Event {
        self.event
    }

    /// Returns a mutable reference to the triggered event.
    pub fn event_mut(&mut self) -> &mut E::Event {
        self.event
    }

    /// Returns a pointer to the triggered event.
    pub fn event_ptr(&self) -> Ptr<'_> {
        Ptr::from(&self.event)
    }

    /// Returns the [`Trigger`](crate::event::Trigger) context for this event.
    pub fn trigger(&self) -> &EventPatternTrigger<'t, E> {
        self.trigger
    }

    /// Returns the mutable [`Trigger`](crate::event::Trigger) context for this event.
    pub fn trigger_mut(&mut self) -> &mut EventPatternTrigger<'t, E> {
        self.trigger
    }

    /// Returns the [`Entity`] of the [`Observer`] of the triggered event.
    /// This allows you to despawn the observer, ceasing observation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_ecs::prelude::*;
    ///
    /// #[derive(EntityEvent)]  
    /// struct AssertEvent {
    ///     entity: Entity,
    /// }
    ///
    /// fn assert_observer(event: On<AssertEvent>) {  
    ///     assert_eq!(event.observer(), event.entity);  
    /// }  
    ///
    /// let mut world = World::new();  
    /// let entity = world.spawn(Observer::new(assert_observer)).id();  
    ///
    /// world.trigger(AssertEvent { entity });  
    /// ```
    pub fn observer(&self) -> Entity {
        self.observer
    }

    /// Returns the source code location that triggered this observer, if the `track_location` cargo feature is enabled.
    pub fn caller(&self) -> MaybeLocation {
        self.trigger_context.caller
    }
}

impl<'w, 't, const AUTO_PROPAGATE: bool, E, T> On<'w, 't, E>
where
    E: EventPattern<
        Event: EntityEvent<Trigger<'t> = PropagateEntityTrigger<AUTO_PROPAGATE, E::Event, T>>,
    >,
    T: Traversal<E::Event>,
{
    /// Returns the original [`Entity`] that this [`EntityEvent`] targeted via [`EntityEvent::event_target`] when it was _first_ triggered,
    /// prior to any propagation logic.
    pub fn original_event_target(&self) -> Entity {
        self.trigger.original_event_target
    }

    /// Enables or disables event propagation, allowing the same event to trigger observers on a chain of different entities.
    ///
    /// The path an [`EntityEvent`] will propagate along is specified by the [`Traversal`] component defined in [`PropagateEntityTrigger`].
    ///
    /// [`EntityEvent`] does not propagate by default. To enable propagation, you must:
    /// + Enable propagation in [`EntityEvent`] using `#[entity_event(propagate)]`. See [`EntityEvent`] for details.
    /// + Either call `propagate(true)` in the first observer or in the [`EntityEvent`] derive add `#[entity_event(auto_propagate)]`.
    ///
    /// You can prevent an event from propagating further using `propagate(false)`. This will prevent the event from triggering on the next
    /// [`Entity`] in the [`Traversal`], but note that all remaining observers for the _current_ entity will still run.
    ///
    ///
    /// [`Traversal`]: crate::traversal::Traversal
    pub fn propagate(&mut self, should_propagate: bool) {
        self.trigger.propagate = should_propagate;
    }

    /// Returns the value of the flag that controls event propagation. See [`propagate`] for more information.
    ///
    /// [`propagate`]: On::propagate
    pub fn get_propagate(&self) -> bool {
        self.trigger.propagate
    }
}

impl<'w, 't, E> Debug for On<'w, 't, E>
where
    E: EventPattern,
    E::Event: Event<Trigger<'t>: Debug> + Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("On")
            .field("event", &self.event)
            .field("trigger", &self.trigger)
            .finish()
    }
}

impl<'w, 't, E: EventPattern> Deref for On<'w, 't, E> {
    type Target = E::Event;

    fn deref(&self) -> &Self::Target {
        self.event
    }
}

impl<'w, 't, E: EventPattern> DerefMut for On<'w, 't, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.event
    }
}

/// Metadata about a specific [`Event`] that triggered an observer.
///
/// This information is exposed via methods on [`On`].
pub struct TriggerContext {
    /// The [`EventKey`] the trigger targeted.
    pub event_key: EventKey,
    /// The location of the source code that triggered the observer.
    pub caller: MaybeLocation,
}
