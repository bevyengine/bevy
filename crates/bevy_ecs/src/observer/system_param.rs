//! System parameters for working with observers.

use core::marker::PhantomData;
use core::ops::DerefMut;
use core::{fmt::Debug, ops::Deref};

use bevy_ptr::Ptr;
use smallvec::SmallVec;

use crate::{
    bundle::Bundle, change_detection::MaybeLocation, component::ComponentId, event::EntityEvent,
    prelude::*,
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
pub struct On<'w, E, B: Bundle = ()> {
    event: &'w mut E,
    propagate: &'w mut bool,
    trigger: ObserverTrigger,
    _marker: PhantomData<B>,
}

/// Deprecated in favor of [`On`].
#[deprecated(since = "0.17.0", note = "Renamed to `On`.")]
pub type Trigger<'w, E, B = ()> = On<'w, E, B>;

impl<'w, E, B: Bundle> On<'w, E, B> {
    /// Creates a new instance of [`On`] for the given event and observer information.
    pub fn new(event: &'w mut E, propagate: &'w mut bool, trigger: ObserverTrigger) -> Self {
        Self {
            event,
            propagate,
            trigger,
            _marker: PhantomData,
        }
    }

    /// Returns the event type of this [`On`] instance.
    pub fn event_key(&self) -> EventKey {
        self.trigger.event_key
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

    /// Returns the components that triggered the observer, out of the
    /// components defined in `B`. Does not necessarily include all of them as
    /// `B` acts like an `OR` filter rather than an `AND` filter.
    pub fn components(&self) -> &[ComponentId] {
        &self.trigger.components
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
    /// fn assert_observer(trigger: On<AssertEvent>) {  
    ///     assert_eq!(trigger.observer(), trigger.target());  
    /// }  
    ///
    /// let mut world = World::new();  
    /// let observer = world.spawn(Observer::new(assert_observer)).id();  
    ///
    /// world.trigger_targets(AssertEvent, observer);  
    /// ```
    pub fn observer(&self) -> Entity {
        self.trigger.observer
    }

    /// Returns the source code location that triggered this observer.
    pub fn caller(&self) -> MaybeLocation {
        self.trigger.caller
    }
}

impl<'w, E: EntityEvent, B: Bundle> On<'w, E, B> {
    /// Returns the [`Entity`] that was targeted by the `event` that triggered this observer.
    ///
    /// Note that if event propagation is enabled, this may not be the same as the original target of the event,
    /// which can be accessed via [`On::original_target`].
    ///
    /// If the event was not targeted at a specific entity, this will return [`Entity::PLACEHOLDER`].
    pub fn target(&self) -> Entity {
        self.trigger.current_target.unwrap_or(Entity::PLACEHOLDER)
    }

    /// Returns the original [`Entity`] that the `event` was targeted at when it was first triggered.
    ///
    /// If event propagation is not enabled, this will always return the same value as [`On::target`].
    ///
    /// If the event was not targeted at a specific entity, this will return [`Entity::PLACEHOLDER`].
    pub fn original_target(&self) -> Entity {
        self.trigger.original_target.unwrap_or(Entity::PLACEHOLDER)
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
        *self.propagate = should_propagate;
    }

    /// Returns the value of the flag that controls event propagation. See [`propagate`] for more information.
    ///
    /// [`propagate`]: On::propagate
    pub fn get_propagate(&self) -> bool {
        *self.propagate
    }
}

impl<'w, E: Debug, B: Bundle> Debug for On<'w, E, B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("On")
            .field("event", &self.event)
            .field("propagate", &self.propagate)
            .field("trigger", &self.trigger)
            .field("_marker", &self._marker)
            .finish()
    }
}

impl<'w, E, B: Bundle> Deref for On<'w, E, B> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        self.event
    }
}

impl<'w, E, B: Bundle> DerefMut for On<'w, E, B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.event
    }
}

/// Metadata about a specific [`Event`] that triggered an observer.
///
/// This information is exposed via methods on [`On`].
#[derive(Debug)]
pub struct ObserverTrigger {
    /// The [`Entity`] of the observer handling the trigger.
    pub observer: Entity,
    /// The [`EventKey`] the trigger targeted.
    pub event_key: EventKey,
    /// The [`ComponentId`]s the trigger targeted.
    pub components: SmallVec<[ComponentId; 2]>,
    /// The entity that the entity-event targeted, if any.
    ///
    /// Note that if event propagation is enabled, this may not be the same as [`ObserverTrigger::original_target`].
    pub current_target: Option<Entity>,
    /// The entity that the entity-event was originally targeted at, if any.
    ///
    /// If event propagation is enabled, this will be the first entity that the event was targeted at,
    /// even if the event was propagated to other entities.
    pub original_target: Option<Entity>,
    /// The location of the source code that triggered the observer.
    pub caller: MaybeLocation,
}

impl ObserverTrigger {
    /// Returns the components that the trigger targeted.
    pub fn components(&self) -> &[ComponentId] {
        &self.components
    }
}
