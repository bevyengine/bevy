//! System parameters for working with observers.

use crate::{
    bundle::Bundle,
    change_detection::MaybeLocation,
    event::{Event, EventKey, PropagateEntityTrigger},
    prelude::*,
    traversal::Traversal,
};
use bevy_ptr::Ptr;
use core::{
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

/// A [system parameter] used by an observer to process events. See [`Observer`] and [`Event`] for examples.
///
/// `On` contains the triggered [`Event`] data for a given run of an `Observer`. It also provides access to the
/// [`Trigger`](crate::event::Trigger), which for things like [`EntityEvent`] with a [`PropagateEntityTrigger`],
/// includes control over event propagation.
///
/// The generic `B: Bundle` is used to further specialize the events that this observer is interested in.
/// The entity involved *does not* have to have these components, but the observer will only be
/// triggered if the event matches the components in `B`.
///
/// This is used to to avoid providing a generic argument in your event, as is done for [`Add`]
/// and the other lifecycle events.
///
/// Providing multiple components in this bundle will cause this event to be triggered by any
/// matching component in the bundle,
/// [rather than requiring all of them to be present](https://github.com/bevyengine/bevy/issues/15325).
///
/// [system parameter]: crate::system::SystemParam
// SAFETY WARNING!
// this type must _never_ expose anything with the 'w lifetime
// See the safety discussion on `Trigger` for more details.
pub struct On<'w, 't, E: Event, B: Bundle = ()> {
    observer: Entity,
    // SAFETY WARNING: never expose this 'w lifetime
    event: &'w mut E,
    // SAFETY WARNING: never expose this 'w lifetime
    trigger: &'w mut E::Trigger<'t>,
    // SAFETY WARNING: never expose this 'w lifetime
    trigger_context: &'w TriggerContext,
    _marker: PhantomData<B>,
}

/// Deprecated in favor of [`On`].
#[deprecated(since = "0.17.0", note = "Renamed to `On`.")]
pub type Trigger<'w, 't, E, B = ()> = On<'w, 't, E, B>;

impl<'w, 't, E: Event, B: Bundle> On<'w, 't, E, B> {
    /// Creates a new instance of [`On`] for the given triggered event.
    pub fn new(
        event: &'w mut E,
        observer: Entity,
        trigger: &'w mut E::Trigger<'t>,
        trigger_context: &'w TriggerContext,
    ) -> Self {
        Self {
            event,
            observer,
            trigger,
            trigger_context,
            _marker: PhantomData,
        }
    }

    /// Returns the event type of this [`On`] instance.
    pub fn event_key(&self) -> EventKey {
        self.trigger_context.event_key
    }

    /// Returns a reference to the triggered event.
    pub fn event(&self) -> &E {
        self.event
    }

    /// Returns a mutable reference to the triggered event.
    pub fn event_mut(&mut self) -> &mut E {
        self.event
    }

    /// Returns a pointer to the triggered event.
    pub fn event_ptr(&self) -> Ptr<'_> {
        Ptr::from(&self.event)
    }

    /// Returns the [`Trigger`](crate::event::Trigger) context for this event.
    pub fn trigger(&self) -> &E::Trigger<'t> {
        self.trigger
    }

    /// Returns the mutable [`Trigger`](crate::event::Trigger) context for this event.
    pub fn trigger_mut(&mut self) -> &mut E::Trigger<'t> {
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

impl<'w, 't, E: EntityEvent, B: Bundle> On<'w, 't, E, B> {
    /// A deprecated way to retrieve the entity that this [`EntityEvent`] targeted at.
    ///
    /// Access the event via [`On::event`], then read the entity that the event was targeting.
    /// Prefer using the field name directly for clarity,
    /// but if you are working in a generic context, you can use [`EntityEvent::event_target`].
    #[deprecated(
        since = "0.17.0",
        note = "Call On::event() to access the event, then read the target entity from the event directly."
    )]
    pub fn target(&self) -> Entity {
        self.event.event_target()
    }
}

impl<
        'w,
        't,
        const AUTO_PROPAGATE: bool,
        E: EntityEvent + for<'a> Event<Trigger<'a> = PropagateEntityTrigger<AUTO_PROPAGATE, E, T>>,
        B: Bundle,
        T: Traversal<E>,
    > On<'w, 't, E, B>
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

impl<'w, 't, E: for<'a> Event<Trigger<'a>: Debug> + Debug, B: Bundle> Debug for On<'w, 't, E, B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("On")
            .field("event", &self.event)
            .field("trigger", &self.trigger)
            .field("_marker", &self._marker)
            .finish()
    }
}

impl<'w, 't, E: Event, B: Bundle> Deref for On<'w, 't, E, B> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        self.event
    }
}

impl<'w, 't, E: Event, B: Bundle> DerefMut for On<'w, 't, E, B> {
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
