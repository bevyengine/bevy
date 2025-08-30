//! System parameters for working with observers.

use crate::{
    bundle::Bundle,
    change_detection::MaybeLocation,
    component::ComponentId,
    event::{EntityComponentsTrigger, Event, PropagateEntityTrigger},
    prelude::*,
    traversal::Traversal,
};
use bevy_ptr::Ptr;
use core::{
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

/// Type containing triggered [`Event`] information for a given run of an [`Observer`]. This contains the
/// [`Event`] data itself. It also provides access to the [`Trigger`](crate::event::Trigger), which for things like
/// [`EntityEvent`] with a [`PropagateEntityTrigger`], includes control over event propagation.
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
    /// Creates a new instance of [`On`] for the given triggered event.
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

    /// Returns the [`Trigger`](crate::event::Trigger) context for this event.
    pub fn trigger(&self) -> &E::Trigger<'w> {
        self.trigger
    }

    /// Returns the mutable [`Trigger`](crate::event::Trigger) context for this event.
    pub fn trigger_mut(&mut self) -> &mut E::Trigger<'w> {
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

impl<
        'w,
        const AUTO_PROPAGATE: bool,
        E: EntityEvent + for<'t> Event<Trigger<'t> = PropagateEntityTrigger<AUTO_PROPAGATE, E, T>>,
        B: Bundle,
        T: Traversal<E>,
    > On<'w, E, B>
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

impl<'w, E: EntityEvent + for<'t> Event<Trigger<'t> = EntityComponentsTrigger<'t>>, B: Bundle>
    On<'w, E, B>
{
    /// A list of all components that were triggered for this [`EntityEvent`].
    pub fn triggered_components(&self) -> &[ComponentId] {
        self.trigger.0
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
    pub event_key: EventKey,
    /// The location of the source code that triggered the observer.
    pub caller: MaybeLocation,
}
