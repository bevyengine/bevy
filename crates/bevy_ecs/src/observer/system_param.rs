//! System parameters for working with observers.

use core::{
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use bevy_ptr::Ptr;

use crate::{
    bundle::Bundle,
    change_detection::MaybeLocation,
    event::{Event, PropagateEntityTrigger},
    prelude::*,
    traversal::Traversal,
};

/// Type containing triggered [`Event`] information for a given run of an [`Observer`]. This contains the
/// [`Event`] data itself. If it was triggered for a specific [`Entity`], it includes that as well. It also
/// contains event propagation information. See [`On::propagate`] for more information.
///
/// The generic `B: Bundle` is used to modify the further specialize the events that this observer is interested in.
/// The entity involved *does not* have to have these components, but the observer will only be
/// triggered if the event matches the components in `B`.
///
/// This is used to to avoid providing a generic argument in your event, as is done for [`Add`]
/// and the other lifecycle events.
///
/// Providing multiple components in this bundle will cause this event to be triggered by any
/// matching component in the bundle,
/// [rather than requiring all of them to be present](https://github.com/bevyengine/bevy/issues/15325).
pub struct On<'w, E: Event, B: Bundle = ()> {
    observer: Entity,
    event: &'w mut E,
    trigger: &'w mut E::Trigger<'w>,
    trigger_context: &'w TriggerContext,
    _marker: PhantomData<B>,
}

/// Deprecated in favor of [`On`].
#[deprecated(since = "0.17.0", note = "Renamed to `On`.")]
pub type Trigger<'w, E, B = ()> = On<'w, E, B>;

impl<'w, E: Event, B: Bundle> On<'w, E, B> {
    /// Creates a new instance of [`On`] for the given event and observer information.
    pub fn new(
        event: &'w mut E,
        observer: Entity,
        trigger: &'w mut E::Trigger<'w>,
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

    /// Returns the trigger context for this event.
    pub fn trigger(&self) -> &E::Trigger<'w> {
        self.trigger
    }

    /// Returns the [`Entity`] that observed the triggered event.
    /// This allows you to despawn the observer, ceasing observation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_ecs::prelude::*;
    ///
    /// #[derive(EntityEvent)]  
    /// struct AssertEvent;  
    ///
    /// fn assert_observer(event: On<AssertEvent>) {  
    ///     assert_eq!(event.observer(), event.entity());  
    /// }  
    ///
    /// let mut world = World::new();  
    /// let observer = world.spawn(Observer::new(assert_observer)).id();  
    ///
    /// world.trigger_targets(AssertEvent, observer);  
    /// ```
    pub fn observer(&self) -> Entity {
        self.observer
    }

    /// Returns the source code location that triggered this observer.
    pub fn caller(&self) -> MaybeLocation {
        self.trigger_context.caller
    }
}

impl<
        'w,
        const AUTO_PROPAGATE: bool,
        E: EntityEvent + for<'t> Event<Trigger<'t> = PropagateEntityTrigger<AUTO_PROPAGATE, E, T>>,
        B: Bundle,
        T: Traversal<E>,
    > On<'w, E, B>
{
    /// Returns the original [`Entity`] that the `event` was targeted at when it was first triggered.
    ///
    /// If event propagation is not enabled, this will always return the same value as [`On::entity`].
    pub fn original_entity(&self) -> Entity {
        self.trigger.original_entity
    }

    /// Enables or disables event propagation, allowing the same event to trigger observers on a chain of different entities.
    ///
    /// The path an event will propagate along is specified by its associated [`Traversal`] component. By default, events
    /// use `()` which ends the path immediately and prevents propagation.
    ///
    /// To enable propagation, you must:
    /// + Set [`EntityEvent::Traversal`] to the component you want to propagate along.
    /// + Either call `propagate(true)` in the first observer or set [`EntityEvent::AUTO_PROPAGATE`] to `true`.
    ///
    /// You can prevent an event from propagating further using `propagate(false)`.
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

impl<'w, E: for<'t> Event<Trigger<'t>: Debug> + Debug, B: Bundle> Debug for On<'w, E, B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("On")
            .field("event", &self.event)
            .field("trigger", &self.trigger)
            .field("_marker", &self._marker)
            .finish()
    }
}

impl<'w, E: Event, B: Bundle> Deref for On<'w, E, B> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        self.event
    }
}

impl<'w, E: Event, B: Bundle> DerefMut for On<'w, E, B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.event
    }
}

/// Metadata about a specific [`Event`] that triggered an observer.
///
/// This information is exposed via methods on [`On`].
pub struct TriggerContext {
    /// The [`EventKey`] the trigger targeted.
    pub(crate) event_key: EventKey,
    /// The location of the source code that triggered the observer.
    pub(crate) caller: MaybeLocation,
}

impl TriggerContext {
    pub fn new<E: Event>(world: &mut World, caller: MaybeLocation) -> Self {
        Self {
            event_key: E::register_event_key(world),
            caller,
        }
    }
}
