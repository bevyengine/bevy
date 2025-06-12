use crate::change_detection::MaybeLocation;
use crate::component::ComponentId;
use crate::world::World;
use crate::{component::Component, traversal::Traversal};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use core::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

/// Something that "happens" and can be observed by app logic for push-based event handling.
///
/// Events can be "triggered" on a [`World`], which will then cause any global [`Observer`]
/// watching that event to run. Events can also be triggered for specific [`TriggerTargets`],
/// typically entities, which will only fire off observers watching those targets.
///
/// Events are normally processed immediately after they are triggered, at the next command flush.
/// This makes them suitable for callback-like patterns where various event handlers respond to
/// events immediately, even recursively triggering other events.
///
/// Targeted events support optional propagation from one entity target to another
/// based on the [`Event::Traversal`] type associated with the event.
///
/// Events must be thread-safe.
///
/// ## Derive
///
/// The [`Event`] trait can be derived.
/// Adding `auto_propagate` sets [`Event::AUTO_PROPAGATE`] to true,
/// while adding `traversal = "X"` sets [`Event::Traversal`] to be of type "X".
///
/// ```
/// use bevy_ecs::prelude::*;
///
/// #[derive(Event)]
/// #[event(auto_propagate, traversal = &'static ChildOf)]
/// struct MyEvent;
/// ```
///
/// # Buffered Events
///
/// Sometimes, you may want events to be buffered and deferred instead of processing them immediately.
///
/// The [`BufferedEvent`] trait provides a simpler, pull-based event handling system that allows you
/// to write events using an [`EventWriter`] and read them later using an [`EventReader`].
/// This requires periodically polling the world for new events, but can be more efficient for
/// batch processing a large number of events at once, and allows events to be evaluated at
/// fixed points in the schedule rather than immediately when they are sent.
///
/// See the [`BufferedEvent`] trait for more details.
///
/// [`World`]: crate::world::World
/// [`TriggerTargets`]: crate::observer::TriggerTargets
/// [`Observer`]: crate::observer::Observer
/// [`Events<E>`]: super::Events
/// [`EventReader`]: super::EventReader
/// [`EventWriter`]: super::EventWriter
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an `Event`",
    label = "invalid `Event`",
    note = "consider annotating `{Self}` with `#[derive(Event)]`"
)]
pub trait Event: Send + Sync + 'static {
    /// The component that describes which [`Entity`] to propagate this event to next, when [propagation] is enabled.
    ///
    /// [`Entity`]: crate::entity::Entity
    /// [propagation]: crate::observer::On::propagate
    type Traversal: Traversal<Self>;

    /// When true, this event will always attempt to propagate when [triggered], without requiring a call
    /// to [`On::propagate`].
    ///
    /// [triggered]: crate::system::Commands::trigger_targets
    /// [`On::propagate`]: crate::observer::On::propagate
    const AUTO_PROPAGATE: bool = false;

    /// Generates the [`ComponentId`] for this event type.
    ///
    /// If this type has already been registered,
    /// this will return the existing [`ComponentId`].
    ///
    /// This is used by various dynamically typed observer APIs,
    /// such as [`World::trigger_targets_dynamic`].
    ///
    /// # Warning
    ///
    /// This method should not be overridden by implementers,
    /// and should always correspond to the implementation of [`component_id`](Event::component_id).
    fn register_component_id(world: &mut World) -> ComponentId {
        world.register_component::<EventWrapperComponent<Self>>()
    }

    /// Fetches the [`ComponentId`] for this event type,
    /// if it has already been generated.
    ///
    /// This is used by various dynamically typed observer APIs,
    /// such as [`World::trigger_targets_dynamic`].
    ///
    /// # Warning
    ///
    /// This method should not be overridden by implementers,
    /// and should always correspond to the implementation of [`register_component_id`](Event::register_component_id).
    fn component_id(world: &World) -> Option<ComponentId> {
        world.component_id::<EventWrapperComponent<Self>>()
    }
}

/// A buffered event for pull-based event handling.
///
/// Buffered events can be written with [`EventWriter`] and read using the [`EventReader`] system parameter.
/// These events are stored in the [`Events<E>`] resource, and require periodically polling the world for new events,
/// typically in a system that runs as part of a schedule.
///
/// While the polling imposes a small overhead, buffered events are useful for efficiently batch processing
/// a large number of events at once. This can make them more efficient than [`Event`]s used by [`Observer`]s
/// for events that happen at a high frequency or in large quantities.
///
/// Unlike [`Event`]s used by observers, buffered events are evaluated at fixed points in the schedule
/// rather than immediately when they are sent. This allows for more predictable scheduling and deferring
/// event processing to a later point in time.
///
/// Buffered events do *not* trigger observers automatically when they are written.
/// However, it is possible to derive both [`Event`] and [`BufferedEvent`] for the same type
/// in case you want both buffered and immediate event handling for the same event.
///
/// Buffered events must be thread-safe.
///
/// # Usage
///
/// The [`BufferedEvent`] trait can be easily derived:
///
/// ```
/// use bevy_ecs::prelude::*;
///
/// #[derive(BufferedEvent, Debug)]
/// struct MyEvent;
/// ```
///
/// The event can then be written to the event buffer using an [`EventWriter`]:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(BufferedEvent)]
/// # struct MyEvent;
/// #
/// fn my_system(mut writer: EventWriter<MyEvent>) {
///     writer.write(MyEvent);
/// }
/// ```
///
/// And read using an [`EventReader`]:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(BufferedEvent)]
/// # struct MyEvent;
/// #
/// fn my_system(mut reader: EventReader<MyEvent>) {
///     // Process the events
///     for event in reader.read() {
///         println!("Received event: {:?}", event);
///     }
/// }
/// ```
///
/// [`World`]: crate::world::World
/// [`Observer`]: crate::observer::Observer
/// [`Events<E>`]: super::Events
/// [`EventReader`]: super::EventReader
/// [`EventWriter`]: super::EventWriter
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an `BufferedEvent`",
    label = "invalid `BufferedEvent`",
    note = "consider annotating `{Self}` with `#[derive(BufferedEvent)]`"
)]
pub trait BufferedEvent: Send + Sync + 'static {}

/// An internal type that implements [`Component`] for a given [`Event`] type.
///
/// This exists so we can easily get access to a unique [`ComponentId`] for each [`Event`] type,
/// without requiring that [`Event`] types implement [`Component`] directly.
/// [`ComponentId`] is used internally as a unique identifier for events because they are:
///
/// - Unique to each event type.
/// - Can be quickly generated and looked up.
/// - Are compatible with dynamic event types, which aren't backed by a Rust type.
///
/// This type is an implementation detail and should never be made public.
// TODO: refactor events to store their metadata on distinct entities, rather than using `ComponentId`
#[derive(Component)]
struct EventWrapperComponent<E: Event + ?Sized>(PhantomData<E>);

/// An `EventId` uniquely identifies an event stored in a specific [`World`].
///
/// An `EventId` can among other things be used to trace the flow of an event from the point it was
/// sent to the point it was processed. `EventId`s increase monotonically by send order.
///
/// [`World`]: crate::world::World
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Clone, Debug, PartialEq, Hash)
)]
pub struct EventId<E: BufferedEvent> {
    /// Uniquely identifies the event associated with this ID.
    // This value corresponds to the order in which each event was added to the world.
    pub id: usize,
    /// The source code location that triggered this event.
    pub caller: MaybeLocation,
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore, clone))]
    pub(super) _marker: PhantomData<E>,
}

impl<E: BufferedEvent> Copy for EventId<E> {}

impl<E: BufferedEvent> Clone for EventId<E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<E: BufferedEvent> fmt::Display for EventId<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

impl<E: BufferedEvent> fmt::Debug for EventId<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "event<{}>#{}",
            core::any::type_name::<E>().split("::").last().unwrap(),
            self.id,
        )
    }
}

impl<E: BufferedEvent> PartialEq for EventId<E> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<E: BufferedEvent> Eq for EventId<E> {}

impl<E: BufferedEvent> PartialOrd for EventId<E> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<E: BufferedEvent> Ord for EventId<E> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl<E: BufferedEvent> Hash for EventId<E> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self.id, state);
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub(crate) struct EventInstance<E: BufferedEvent> {
    pub event_id: EventId<E>,
    pub event: E,
}
