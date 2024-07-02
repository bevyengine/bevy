//! Event handling types.

use crate as bevy_ecs;
#[cfg(feature = "multi_threaded")]
use crate::batching::BatchingStrategy;
use crate::change_detection::MutUntyped;
use crate::{
    change_detection::{DetectChangesMut, Mut},
    component::{Component, ComponentId, Tick},
    system::{Local, Res, ResMut, Resource, SystemParam},
    world::World,
};
pub use bevy_ecs_macros::Event;
use bevy_ecs_macros::SystemSet;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use bevy_utils::detailed_trace;
use std::ops::{Deref, DerefMut};
use std::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    iter::Chain,
    marker::PhantomData,
    slice::Iter,
};

/// Something that "happens" and might be read / observed by app logic.
///
/// Events can be stored in an [`Events<E>`] resource
/// You can conveniently access events using the [`EventReader`] and [`EventWriter`] system parameter.
///
/// Events can also be "triggered" on a [`World`], which will then cause any [`Observer`] of that trigger to run.
///
/// This trait can be derived.
///
/// Events implement the [`Component`] type (and they automatically do when they are derived). Events are (generally)
/// not directly inserted as components. More often, the [`ComponentId`] is used to identify the event type within the
/// context of the ECS.
///
/// Events must be thread-safe.
///
/// [`World`]: crate::world::World
/// [`ComponentId`]: crate::component::ComponentId
/// [`Observer`]: crate::observer::Observer
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an `Event`",
    label = "invalid `Event`",
    note = "consider annotating `{Self}` with `#[derive(Event)]`"
)]
pub trait Event: Component {}

/// An `EventId` uniquely identifies an event stored in a specific [`World`].
///
/// An `EventId` can among other things be used to trace the flow of an event from the point it was
/// sent to the point it was processed. `EventId`s increase monotonically by send order.
///
/// [`World`]: crate::world::World
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct EventId<E: Event> {
    /// Uniquely identifies the event associated with this ID.
    // This value corresponds to the order in which each event was added to the world.
    pub id: usize,
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    _marker: PhantomData<E>,
}

impl<E: Event> Copy for EventId<E> {}

impl<E: Event> Clone for EventId<E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<E: Event> fmt::Display for EventId<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

impl<E: Event> fmt::Debug for EventId<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "event<{}>#{}",
            std::any::type_name::<E>().split("::").last().unwrap(),
            self.id,
        )
    }
}

impl<E: Event> PartialEq for EventId<E> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<E: Event> Eq for EventId<E> {}

impl<E: Event> PartialOrd for EventId<E> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<E: Event> Ord for EventId<E> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl<E: Event> Hash for EventId<E> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self.id, state);
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
struct EventInstance<E: Event> {
    pub event_id: EventId<E>,
    pub event: E,
}

/// An event collection that represents the events that occurred within the last two
/// [`Events::update`] calls.
/// Events can be written to using an [`EventWriter`]
/// and are typically cheaply read using an [`EventReader`].
///
/// Each event can be consumed by multiple systems, in parallel,
/// with consumption tracked by the [`EventReader`] on a per-system basis.
///
/// If no [ordering](https://github.com/bevyengine/bevy/blob/main/examples/ecs/ecs_guide.rs)
/// is applied between writing and reading systems, there is a risk of a race condition.
/// This means that whether the events arrive before or after the next [`Events::update`] is unpredictable.
///
/// This collection is meant to be paired with a system that calls
/// [`Events::update`] exactly once per update/frame.
///
/// [`event_update_system`] is a system that does this, typically initialized automatically using
/// [`add_event`](https://docs.rs/bevy/*/bevy/app/struct.App.html#method.add_event).
/// [`EventReader`]s are expected to read events from this collection at least once per loop/frame.
/// Events will persist across a single frame boundary and so ordering of event producers and
/// consumers is not critical (although poorly-planned ordering may cause accumulating lag).
/// If events are not handled by the end of the frame after they are updated, they will be
/// dropped silently.
///
/// # Example
/// ```
/// use bevy_ecs::event::{Event, Events};
///
/// #[derive(Event)]
/// struct MyEvent {
///     value: usize
/// }
///
/// // setup
/// let mut events = Events::<MyEvent>::default();
/// let mut reader = events.get_reader();
///
/// // run this once per update/frame
/// events.update();
///
/// // somewhere else: send an event
/// events.send(MyEvent { value: 1 });
///
/// // somewhere else: read the events
/// for event in reader.read(&events) {
///     assert_eq!(event.value, 1)
/// }
///
/// // events are only processed once per reader
/// assert_eq!(reader.read(&events).count(), 0);
/// ```
///
/// # Details
///
/// [`Events`] is implemented using a variation of a double buffer strategy.
/// Each call to [`update`](Events::update) swaps buffers and clears out the oldest one.
/// - [`EventReader`]s will read events from both buffers.
/// - [`EventReader`]s that read at least once per update will never drop events.
/// - [`EventReader`]s that read once within two updates might still receive some events
/// - [`EventReader`]s that read after two updates are guaranteed to drop all events that occurred
/// before those updates.
///
/// The buffers in [`Events`] will grow indefinitely if [`update`](Events::update) is never called.
///
/// An alternative call pattern would be to call [`update`](Events::update)
/// manually across frames to control when events are cleared.
/// This complicates consumption and risks ever-expanding memory usage if not cleaned up,
/// but can be done by adding your event as a resource instead of using
/// [`add_event`](https://docs.rs/bevy/*/bevy/app/struct.App.html#method.add_event).
///
/// [Example usage.](https://github.com/bevyengine/bevy/blob/latest/examples/ecs/event.rs)
/// [Example usage standalone.](https://github.com/bevyengine/bevy/blob/latest/crates/bevy_ecs/examples/events.rs)
///
#[derive(Debug, Resource)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct Events<E: Event> {
    /// Holds the oldest still active events.
    /// Note that `a.start_event_count + a.len()` should always be equal to `events_b.start_event_count`.
    events_a: EventSequence<E>,
    /// Holds the newer events.
    events_b: EventSequence<E>,
    event_count: usize,
}

// Derived Default impl would incorrectly require E: Default
impl<E: Event> Default for Events<E> {
    fn default() -> Self {
        Self {
            events_a: Default::default(),
            events_b: Default::default(),
            event_count: Default::default(),
        }
    }
}

impl<E: Event> Events<E> {
    /// Returns the index of the oldest event stored in the event buffer.
    pub fn oldest_event_count(&self) -> usize {
        self.events_a
            .start_event_count
            .min(self.events_b.start_event_count)
    }

    /// "Sends" an `event` by writing it to the current event buffer. [`EventReader`]s can then read
    /// the event.
    /// This method returns the [ID](`EventId`) of the sent `event`.
    pub fn send(&mut self, event: E) -> EventId<E> {
        let event_id = EventId {
            id: self.event_count,
            _marker: PhantomData,
        };
        detailed_trace!("Events::send() -> id: {}", event_id);

        let event_instance = EventInstance { event_id, event };

        self.events_b.push(event_instance);
        self.event_count += 1;

        event_id
    }

    /// Sends a list of `events` all at once, which can later be read by [`EventReader`]s.
    /// This is more efficient than sending each event individually.
    /// This method returns the [IDs](`EventId`) of the sent `events`.
    pub fn send_batch(&mut self, events: impl IntoIterator<Item = E>) -> SendBatchIds<E> {
        let last_count = self.event_count;

        self.extend(events);

        SendBatchIds {
            last_count,
            event_count: self.event_count,
            _marker: PhantomData,
        }
    }

    /// Sends the default value of the event. Useful when the event is an empty struct.
    /// This method returns the [ID](`EventId`) of the sent `event`.
    pub fn send_default(&mut self) -> EventId<E>
    where
        E: Default,
    {
        self.send(Default::default())
    }

    /// Gets a new [`ManualEventReader`]. This will include all events already in the event buffers.
    pub fn get_reader(&self) -> ManualEventReader<E> {
        ManualEventReader::default()
    }

    /// Gets a new [`ManualEventReader`]. This will ignore all events already in the event buffers.
    /// It will read all future events.
    pub fn get_reader_current(&self) -> ManualEventReader<E> {
        ManualEventReader {
            last_event_count: self.event_count,
            ..Default::default()
        }
    }

    /// Swaps the event buffers and clears the oldest event buffer. In general, this should be
    /// called once per frame/update.
    ///
    /// If you need access to the events that were removed, consider using [`Events::update_drain`].
    pub fn update(&mut self) {
        std::mem::swap(&mut self.events_a, &mut self.events_b);
        self.events_b.clear();
        self.events_b.start_event_count = self.event_count;
        debug_assert_eq!(
            self.events_a.start_event_count + self.events_a.len(),
            self.events_b.start_event_count
        );
    }

    /// Swaps the event buffers and drains the oldest event buffer, returning an iterator
    /// of all events that were removed. In general, this should be called once per frame/update.
    ///
    /// If you do not need to take ownership of the removed events, use [`Events::update`] instead.
    #[must_use = "If you do not need the returned events, call .update() instead."]
    pub fn update_drain(&mut self) -> impl Iterator<Item = E> + '_ {
        std::mem::swap(&mut self.events_a, &mut self.events_b);
        let iter = self.events_b.events.drain(..);
        self.events_b.start_event_count = self.event_count;
        debug_assert_eq!(
            self.events_a.start_event_count + self.events_a.len(),
            self.events_b.start_event_count
        );

        iter.map(|e| e.event)
    }

    #[inline]
    fn reset_start_event_count(&mut self) {
        self.events_a.start_event_count = self.event_count;
        self.events_b.start_event_count = self.event_count;
    }

    /// Removes all events.
    #[inline]
    pub fn clear(&mut self) {
        self.reset_start_event_count();
        self.events_a.clear();
        self.events_b.clear();
    }

    /// Returns the number of events currently stored in the event buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.events_a.len() + self.events_b.len()
    }

    /// Returns true if there are no events currently stored in the event buffer.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Creates a draining iterator that removes all events.
    pub fn drain(&mut self) -> impl Iterator<Item = E> + '_ {
        self.reset_start_event_count();

        // Drain the oldest events first, then the newest
        self.events_a
            .drain(..)
            .chain(self.events_b.drain(..))
            .map(|i| i.event)
    }

    /// Iterates over events that happened since the last "update" call.
    /// WARNING: You probably don't want to use this call. In most cases you should use an
    /// [`EventReader`]. You should only use this if you know you only need to consume events
    /// between the last `update()` call and your call to `iter_current_update_events`.
    /// If events happen outside that window, they will not be handled. For example, any events that
    /// happen after this call and before the next `update()` call will be dropped.
    pub fn iter_current_update_events(&self) -> impl ExactSizeIterator<Item = &E> {
        self.events_b.iter().map(|i| &i.event)
    }

    /// Get a specific event by id if it still exists in the events buffer.
    pub fn get_event(&self, id: usize) -> Option<(&E, EventId<E>)> {
        if id < self.oldest_id() {
            return None;
        }

        let sequence = self.sequence(id);
        let index = id.saturating_sub(sequence.start_event_count);

        sequence
            .get(index)
            .map(|instance| (&instance.event, instance.event_id))
    }

    /// Oldest id still in the events buffer.
    pub fn oldest_id(&self) -> usize {
        self.events_a.start_event_count
    }

    /// Which event buffer is this event id a part of.
    fn sequence(&self, id: usize) -> &EventSequence<E> {
        if id < self.events_b.start_event_count {
            &self.events_a
        } else {
            &self.events_b
        }
    }
}

impl<E: Event> Extend<E> for Events<E> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = E>,
    {
        let old_count = self.event_count;
        let mut event_count = self.event_count;
        let events = iter.into_iter().map(|event| {
            let event_id = EventId {
                id: event_count,
                _marker: PhantomData,
            };
            event_count += 1;
            EventInstance { event_id, event }
        });

        self.events_b.extend(events);

        if old_count != event_count {
            detailed_trace!(
                "Events::extend() -> ids: ({}..{})",
                self.event_count,
                event_count
            );
        }

        self.event_count = event_count;
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
struct EventSequence<E: Event> {
    events: Vec<EventInstance<E>>,
    start_event_count: usize,
}

// Derived Default impl would incorrectly require E: Default
impl<E: Event> Default for EventSequence<E> {
    fn default() -> Self {
        Self {
            events: Default::default(),
            start_event_count: Default::default(),
        }
    }
}

impl<E: Event> Deref for EventSequence<E> {
    type Target = Vec<EventInstance<E>>;

    fn deref(&self) -> &Self::Target {
        &self.events
    }
}

impl<E: Event> DerefMut for EventSequence<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.events
    }
}

/// Reads events of type `T` in order and tracks which events have already been read.
///
/// # Concurrency
///
/// Unlike [`EventWriter<T>`], systems with `EventReader<T>` param can be executed concurrently
/// (but not concurrently with `EventWriter<T>` systems for the same event type).
#[derive(SystemParam, Debug)]
pub struct EventReader<'w, 's, E: Event> {
    reader: Local<'s, ManualEventReader<E>>,
    events: Res<'w, Events<E>>,
}

impl<'w, 's, E: Event> EventReader<'w, 's, E> {
    /// Iterates over the events this [`EventReader`] has not seen yet. This updates the
    /// [`EventReader`]'s event counter, which means subsequent event reads will not include events
    /// that happened before now.
    pub fn read(&mut self) -> EventIterator<'_, E> {
        self.reader.read(&self.events)
    }

    /// Like [`read`](Self::read), except also returning the [`EventId`] of the events.
    pub fn read_with_id(&mut self) -> EventIteratorWithId<'_, E> {
        self.reader.read_with_id(&self.events)
    }

    /// Returns a parallel iterator over the events this [`EventReader`] has not seen yet.
    /// See also [`for_each`](EventParIter::for_each).
    ///
    /// # Example
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use std::sync::atomic::{AtomicUsize, Ordering};
    ///
    /// #[derive(Event)]
    /// struct MyEvent {
    ///     value: usize,
    /// }
    ///
    /// #[derive(Resource, Default)]
    /// struct Counter(AtomicUsize);
    ///
    /// // setup
    /// let mut world = World::new();
    /// world.init_resource::<Events<MyEvent>>();
    /// world.insert_resource(Counter::default());
    ///
    /// let mut schedule = Schedule::default();
    /// schedule.add_systems(|mut events: EventReader<MyEvent>, counter: Res<Counter>| {
    ///     events.par_read().for_each(|MyEvent { value }| {
    ///         counter.0.fetch_add(*value, Ordering::Relaxed);
    ///     });
    /// });
    /// for value in 0..100 {
    ///     world.send_event(MyEvent { value });
    /// }
    /// schedule.run(&mut world);
    /// let Counter(counter) = world.remove_resource::<Counter>().unwrap();
    /// // all events were processed
    /// assert_eq!(counter.into_inner(), 4950);
    /// ```
    ///
    #[cfg(feature = "multi_threaded")]
    pub fn par_read(&mut self) -> EventParIter<'_, E> {
        self.reader.par_read(&self.events)
    }

    /// Determines the number of events available to be read from this [`EventReader`] without consuming any.
    pub fn len(&self) -> usize {
        self.reader.len(&self.events)
    }

    /// Returns `true` if there are no events available to read.
    ///
    /// # Example
    ///
    /// The following example shows a useful pattern where some behavior is triggered if new events are available.
    /// [`EventReader::clear()`] is used so the same events don't re-trigger the behavior the next time the system runs.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// #[derive(Event)]
    /// struct CollisionEvent;
    ///
    /// fn play_collision_sound(mut events: EventReader<CollisionEvent>) {
    ///     if !events.is_empty() {
    ///         events.clear();
    ///         // Play a sound
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(play_collision_sound);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.reader.is_empty(&self.events)
    }

    /// Consumes all available events.
    ///
    /// This means these events will not appear in calls to [`EventReader::read()`] or
    /// [`EventReader::read_with_id()`] and [`EventReader::is_empty()`] will return `true`.
    ///
    /// For usage, see [`EventReader::is_empty()`].
    pub fn clear(&mut self) {
        self.reader.clear(&self.events);
    }
}

/// Sends events of type `T`.
///
/// # Usage
///
/// `EventWriter`s are usually declared as a [`SystemParam`].
/// ```
/// # use bevy_ecs::prelude::*;
///
/// #[derive(Event)]
/// pub struct MyEvent; // Custom event type.
/// fn my_system(mut writer: EventWriter<MyEvent>) {
///     writer.send(MyEvent);
/// }
///
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
/// # Observers
///
/// "Buffered" Events, such as those sent directly in [`Events`] or sent using [`EventWriter`], do _not_ automatically
/// trigger any [`Observer`]s watching for that event, as each [`Event`] has different requirements regarding _if_ it will
/// be triggered, and if so, _when_ it will be triggered in the schedule.
///
/// # Concurrency
///
/// `EventWriter` param has [`ResMut<Events<T>>`](Events) inside. So two systems declaring `EventWriter<T>` params
/// for the same event type won't be executed concurrently.
///
/// # Untyped events
///
/// `EventWriter` can only send events of one specific type, which must be known at compile-time.
/// This is not a problem most of the time, but you may find a situation where you cannot know
/// ahead of time every kind of event you'll need to send. In this case, you can use the "type-erased event" pattern.
///
/// ```
/// # use bevy_ecs::{prelude::*, event::Events};
/// # #[derive(Event)]
/// # pub struct MyEvent;
/// fn send_untyped(mut commands: Commands) {
///     // Send an event of a specific type without having to declare that
///     // type as a SystemParam.
///     //
///     // Effectively, we're just moving the type parameter from the /type/ to the /method/,
///     // which allows one to do all kinds of clever things with type erasure, such as sending
///     // custom events to unknown 3rd party plugins (modding API).
///     //
///     // NOTE: the event won't actually be sent until commands get applied during
///     // apply_deferred.
///     commands.add(|w: &mut World| {
///         w.send_event(MyEvent);
///     });
/// }
/// ```
/// Note that this is considered *non-idiomatic*, and should only be used when `EventWriter` will not work.
///
/// [`Observer`]: crate::observer::Observer
#[derive(SystemParam)]
pub struct EventWriter<'w, E: Event> {
    events: ResMut<'w, Events<E>>,
}

impl<'w, E: Event> EventWriter<'w, E> {
    /// Sends an `event`, which can later be read by [`EventReader`]s.
    /// This method returns the [ID](`EventId`) of the sent `event`.
    ///
    /// See [`Events`] for details.
    pub fn send(&mut self, event: E) -> EventId<E> {
        self.events.send(event)
    }

    /// Sends a list of `events` all at once, which can later be read by [`EventReader`]s.
    /// This is more efficient than sending each event individually.
    /// This method returns the [IDs](`EventId`) of the sent `events`.
    ///
    /// See [`Events`] for details.
    pub fn send_batch(&mut self, events: impl IntoIterator<Item = E>) -> SendBatchIds<E> {
        self.events.send_batch(events)
    }

    /// Sends the default value of the event. Useful when the event is an empty struct.
    /// This method returns the [ID](`EventId`) of the sent `event`.
    ///
    /// See [`Events`] for details.
    pub fn send_default(&mut self) -> EventId<E>
    where
        E: Default,
    {
        self.events.send_default()
    }
}

/// Stores the state for an [`EventReader`].
///
/// Access to the [`Events<E>`] resource is required to read any incoming events.
///
/// In almost all cases, you should just use an [`EventReader`],
/// which will automatically manage the state for you.
///
/// However, this type can be useful if you need to manually track events,
/// such as when you're attempting to send and receive events of the same type in the same system.
///
/// # Example
///
/// ```
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::event::{Event, Events, ManualEventReader};
///
/// #[derive(Event, Clone, Debug)]
/// struct MyEvent;
///
/// /// A system that both sends and receives events using a [`Local`] [`ManualEventReader`].
/// fn send_and_receive_manual_event_reader(
///     // The `Local` `SystemParam` stores state inside the system itself, rather than in the world.
///     // `ManualEventReader<T>` is the internal state of `EventReader<T>`, which tracks which events have been seen.
///     mut local_event_reader: Local<ManualEventReader<MyEvent>>,
///     // We can access the `Events` resource mutably, allowing us to both read and write its contents.
///     mut events: ResMut<Events<MyEvent>>,
/// ) {
///     // We must collect the events to resend, because we can't mutate events while we're iterating over the events.
///     let mut events_to_resend = Vec::new();
///
///     for event in local_event_reader.read(&events) {
///          events_to_resend.push(event.clone());
///     }
///
///     for event in events_to_resend {
///         events.send(MyEvent);
///     }
/// }
///
/// # bevy_ecs::system::assert_is_system(send_and_receive_manual_event_reader);
/// ```
#[derive(Debug)]
pub struct ManualEventReader<E: Event> {
    last_event_count: usize,
    _marker: PhantomData<E>,
}

impl<E: Event> Default for ManualEventReader<E> {
    fn default() -> Self {
        ManualEventReader {
            last_event_count: 0,
            _marker: Default::default(),
        }
    }
}

impl<E: Event> Clone for ManualEventReader<E> {
    fn clone(&self) -> Self {
        ManualEventReader {
            last_event_count: self.last_event_count,
            _marker: PhantomData,
        }
    }
}

#[allow(clippy::len_without_is_empty)] // Check fails since the is_empty implementation has a signature other than `(&self) -> bool`
impl<E: Event> ManualEventReader<E> {
    /// See [`EventReader::read`]
    pub fn read<'a>(&'a mut self, events: &'a Events<E>) -> EventIterator<'a, E> {
        self.read_with_id(events).without_id()
    }

    /// See [`EventReader::read_with_id`]
    pub fn read_with_id<'a>(&'a mut self, events: &'a Events<E>) -> EventIteratorWithId<'a, E> {
        EventIteratorWithId::new(self, events)
    }

    /// See [`EventReader::par_read`]
    #[cfg(feature = "multi_threaded")]
    pub fn par_read<'a>(&'a mut self, events: &'a Events<E>) -> EventParIter<'a, E> {
        EventParIter::new(self, events)
    }

    /// See [`EventReader::len`]
    pub fn len(&self, events: &Events<E>) -> usize {
        // The number of events in this reader is the difference between the most recent event
        // and the last event seen by it. This will be at most the number of events contained
        // with the events (any others have already been dropped)
        // TODO: Warn when there are dropped events, or return e.g. a `Result<usize, (usize, usize)>`
        events
            .event_count
            .saturating_sub(self.last_event_count)
            .min(events.len())
    }

    /// Amount of events we missed.
    pub fn missed_events(&self, events: &Events<E>) -> usize {
        events
            .oldest_event_count()
            .saturating_sub(self.last_event_count)
    }

    /// See [`EventReader::is_empty()`]
    pub fn is_empty(&self, events: &Events<E>) -> bool {
        self.len(events) == 0
    }

    /// See [`EventReader::clear()`]
    pub fn clear(&mut self, events: &Events<E>) {
        self.last_event_count = events.event_count;
    }
}

/// An iterator that yields any unread events from an [`EventReader`] or [`ManualEventReader`].
#[derive(Debug)]
pub struct EventIterator<'a, E: Event> {
    iter: EventIteratorWithId<'a, E>,
}

impl<'a, E: Event> Iterator for EventIterator<'a, E> {
    type Item = &'a E;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(event, _)| event)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn count(self) -> usize {
        self.iter.count()
    }

    fn last(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        self.iter.last().map(|(event, _)| event)
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.iter.nth(n).map(|(event, _)| event)
    }
}

impl<'a, E: Event> ExactSizeIterator for EventIterator<'a, E> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

/// An iterator that yields any unread events (and their IDs) from an [`EventReader`] or [`ManualEventReader`].
#[derive(Debug)]
pub struct EventIteratorWithId<'a, E: Event> {
    reader: &'a mut ManualEventReader<E>,
    chain: Chain<Iter<'a, EventInstance<E>>, Iter<'a, EventInstance<E>>>,
    unread: usize,
}

impl<'a, E: Event> EventIteratorWithId<'a, E> {
    /// Creates a new iterator that yields any `events` that have not yet been seen by `reader`.
    pub fn new(reader: &'a mut ManualEventReader<E>, events: &'a Events<E>) -> Self {
        let a_index = reader
            .last_event_count
            .saturating_sub(events.events_a.start_event_count);
        let b_index = reader
            .last_event_count
            .saturating_sub(events.events_b.start_event_count);
        let a = events.events_a.get(a_index..).unwrap_or_default();
        let b = events.events_b.get(b_index..).unwrap_or_default();

        let unread_count = a.len() + b.len();
        // Ensure `len` is implemented correctly
        debug_assert_eq!(unread_count, reader.len(events));
        reader.last_event_count = events.event_count - unread_count;
        // Iterate the oldest first, then the newer events
        let chain = a.iter().chain(b.iter());

        Self {
            reader,
            chain,
            unread: unread_count,
        }
    }

    /// Iterate over only the events.
    pub fn without_id(self) -> EventIterator<'a, E> {
        EventIterator { iter: self }
    }
}

impl<'a, E: Event> Iterator for EventIteratorWithId<'a, E> {
    type Item = (&'a E, EventId<E>);
    fn next(&mut self) -> Option<Self::Item> {
        match self
            .chain
            .next()
            .map(|instance| (&instance.event, instance.event_id))
        {
            Some(item) => {
                detailed_trace!("EventReader::iter() -> {}", item.1);
                self.reader.last_event_count += 1;
                self.unread -= 1;
                Some(item)
            }
            None => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.chain.size_hint()
    }

    fn count(self) -> usize {
        self.reader.last_event_count += self.unread;
        self.unread
    }

    fn last(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        let EventInstance { event_id, event } = self.chain.last()?;
        self.reader.last_event_count += self.unread;
        Some((event, *event_id))
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        if let Some(EventInstance { event_id, event }) = self.chain.nth(n) {
            self.reader.last_event_count += n + 1;
            self.unread -= n + 1;
            Some((event, *event_id))
        } else {
            self.reader.last_event_count += self.unread;
            self.unread = 0;
            None
        }
    }
}

impl<'a, E: Event> ExactSizeIterator for EventIteratorWithId<'a, E> {
    fn len(&self) -> usize {
        self.unread
    }
}

/// A parallel iterator over `Event`s.
#[cfg(feature = "multi_threaded")]
#[derive(Debug)]
pub struct EventParIter<'a, E: Event> {
    reader: &'a mut ManualEventReader<E>,
    slices: [&'a [EventInstance<E>]; 2],
    batching_strategy: BatchingStrategy,
    unread: usize,
}

#[cfg(feature = "multi_threaded")]
impl<'a, E: Event> EventParIter<'a, E> {
    /// Creates a new parallel iterator over `events` that have not yet been seen by `reader`.
    pub fn new(reader: &'a mut ManualEventReader<E>, events: &'a Events<E>) -> Self {
        let a_index = reader
            .last_event_count
            .saturating_sub(events.events_a.start_event_count);
        let b_index = reader
            .last_event_count
            .saturating_sub(events.events_b.start_event_count);
        let a = events.events_a.get(a_index..).unwrap_or_default();
        let b = events.events_b.get(b_index..).unwrap_or_default();

        let unread_count = a.len() + b.len();
        // Ensure `len` is implemented correctly
        debug_assert_eq!(unread_count, reader.len(events));
        reader.last_event_count = events.event_count - unread_count;

        Self {
            reader,
            slices: [a, b],
            batching_strategy: BatchingStrategy::default(),
            unread: unread_count,
        }
    }

    /// Changes the batching strategy used when iterating.
    ///
    /// For more information on how this affects the resultant iteration, see
    /// [`BatchingStrategy`].
    pub fn batching_strategy(mut self, strategy: BatchingStrategy) -> Self {
        self.batching_strategy = strategy;
        self
    }

    /// Runs the provided closure for each unread event in parallel.
    ///
    /// Unlike normal iteration, the event order is not guaranteed in any form.
    ///
    /// # Panics
    /// If the [`ComputeTaskPool`] is not initialized. If using this from an event reader that is being
    /// initialized and run from the ECS scheduler, this should never panic.
    ///
    /// [`ComputeTaskPool`]: bevy_tasks::ComputeTaskPool
    pub fn for_each<FN: Fn(&'a E) + Send + Sync + Clone>(self, func: FN) {
        self.for_each_with_id(move |e, _| func(e));
    }

    /// Runs the provided closure for each unread event in parallel, like [`for_each`](Self::for_each),
    /// but additionally provides the `EventId` to the closure.
    ///
    /// Note that the order of iteration is not guaranteed, but `EventId`s are ordered by send order.
    ///
    /// # Panics
    /// If the [`ComputeTaskPool`] is not initialized. If using this from an event reader that is being
    /// initialized and run from the ECS scheduler, this should never panic.
    ///
    /// [`ComputeTaskPool`]: bevy_tasks::ComputeTaskPool
    pub fn for_each_with_id<FN: Fn(&'a E, EventId<E>) + Send + Sync + Clone>(mut self, func: FN) {
        #[cfg(any(target_arch = "wasm32", not(feature = "multi_threaded")))]
        {
            self.into_iter().for_each(|(e, i)| func(e, i));
        }

        #[cfg(all(not(target_arch = "wasm32"), feature = "multi_threaded"))]
        {
            let pool = bevy_tasks::ComputeTaskPool::get();
            let thread_count = pool.thread_num();
            if thread_count <= 1 {
                return self.into_iter().for_each(|(e, i)| func(e, i));
            }

            let batch_size = self
                .batching_strategy
                .calc_batch_size(|| self.len(), thread_count);
            let chunks = self.slices.map(|s| s.chunks_exact(batch_size));
            let remainders = chunks.each_ref().map(|c| c.remainder());

            pool.scope(|scope| {
                for batch in chunks.into_iter().flatten().chain(remainders) {
                    let func = func.clone();
                    scope.spawn(async move {
                        for event in batch {
                            func(&event.event, event.event_id);
                        }
                    });
                }
            });

            // Events are guaranteed to be read at this point.
            self.reader.last_event_count += self.unread;
            self.unread = 0;
        }
    }

    /// Returns the number of [`Event`]s to be iterated.
    pub fn len(&self) -> usize {
        self.slices.iter().map(|s| s.len()).sum()
    }

    /// Returns [`true`] if there are no events remaining in this iterator.
    pub fn is_empty(&self) -> bool {
        self.slices.iter().all(|x| x.is_empty())
    }
}

#[cfg(feature = "multi_threaded")]
impl<'a, E: Event> IntoIterator for EventParIter<'a, E> {
    type IntoIter = EventIteratorWithId<'a, E>;
    type Item = <Self::IntoIter as Iterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        let EventParIter {
            reader,
            slices: [a, b],
            ..
        } = self;
        let unread = a.len() + b.len();
        let chain = a.iter().chain(b);
        EventIteratorWithId {
            reader,
            chain,
            unread,
        }
    }
}

#[doc(hidden)]
struct RegisteredEvent {
    component_id: ComponentId,
    // Required to flush the secondary buffer and drop events even if left unchanged.
    previously_updated: bool,
    // SAFETY: The component ID and the function must be used to fetch the Events<T> resource
    // of the same type initialized in `register_event`, or improper type casts will occur.
    update: unsafe fn(MutUntyped),
}

/// A registry of all of the [`Events`] in the [`World`], used by [`event_update_system`]
/// to update all of the events.
#[derive(Resource, Default)]
pub struct EventRegistry {
    /// Should the events be updated?
    ///
    /// This field is generally automatically updated by the [`signal_event_update_system`](crate::event::update::signal_event_update_system).
    pub should_update: ShouldUpdateEvents,
    event_updates: Vec<RegisteredEvent>,
}

/// Controls whether or not the events in an [`EventRegistry`] should be updated.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShouldUpdateEvents {
    /// Without any fixed timestep, events should always be updated each frame.
    #[default]
    Always,
    /// We need to wait until at least one pass of the fixed update schedules to update the events.
    Waiting,
    /// At least one pass of the fixed update schedules has occurred, and the events are ready to be updated.
    Ready,
}

impl EventRegistry {
    /// Registers an event type to be updated in a given [`World`]
    ///
    /// If no instance of the [`EventRegistry`] exists in the world, this will add one - otherwise it will use
    /// the existing instance.
    pub fn register_event<T: Event>(world: &mut World) {
        // By initializing the resource here, we can be sure that it is present,
        // and receive the correct, up-to-date `ComponentId` even if it was previously removed.
        let component_id = world.init_resource::<Events<T>>();
        let mut registry = world.get_resource_or_insert_with(Self::default);
        registry.event_updates.push(RegisteredEvent {
            component_id,
            previously_updated: false,
            update: |ptr| {
                // SAFETY: The resource was initialized with the type Events<T>.
                unsafe { ptr.with_type::<Events<T>>() }
                    .bypass_change_detection()
                    .update();
            },
        });
    }

    /// Removes an event from the world and it's associated [`EventRegistry`].
    pub fn deregister_events<T: Event>(world: &mut World) {
        let component_id = world.init_resource::<Events<T>>();
        let mut registry = world.get_resource_or_insert_with(Self::default);
        registry
            .event_updates
            .retain(|e| e.component_id != component_id);
        world.remove_resource::<Events<T>>();
    }

    /// Updates all of the registered events in the World.
    pub fn run_updates(&mut self, world: &mut World, last_change_tick: Tick) {
        for registered_event in &mut self.event_updates {
            // Bypass the type ID -> Component ID lookup with the cached component ID.
            if let Some(events) = world.get_resource_mut_by_id(registered_event.component_id) {
                let has_changed = events.has_changed_since(last_change_tick);
                if registered_event.previously_updated || has_changed {
                    // SAFETY: The update function pointer is called with the resource
                    // fetched from the same component ID.
                    unsafe { (registered_event.update)(events) };
                    // Always set to true if the events have changed, otherwise disable running on the second invocation
                    // to wait for more changes.
                    registered_event.previously_updated =
                        has_changed || !registered_event.previously_updated;
                }
            }
        }
    }
}

#[doc(hidden)]
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EventUpdates;

/// Signals the [`event_update_system`] to run after `FixedUpdate` systems.
///
/// This will change the behavior of the [`EventRegistry`] to only run after a fixed update cycle has passed.
/// Normally, this will simply run every frame.
pub fn signal_event_update_system(signal: Option<ResMut<EventRegistry>>) {
    if let Some(mut registry) = signal {
        registry.should_update = ShouldUpdateEvents::Ready;
    }
}

/// A system that calls [`Events::update`] on all registered [`Events`] in the world.
pub fn event_update_system(world: &mut World, mut last_change_tick: Local<Tick>) {
    if world.contains_resource::<EventRegistry>() {
        world.resource_scope(|world, mut registry: Mut<EventRegistry>| {
            registry.run_updates(world, *last_change_tick);

            registry.should_update = match registry.should_update {
                // If we're always updating, keep doing so.
                ShouldUpdateEvents::Always => ShouldUpdateEvents::Always,
                // Disable the system until signal_event_update_system runs again.
                ShouldUpdateEvents::Waiting | ShouldUpdateEvents::Ready => {
                    ShouldUpdateEvents::Waiting
                }
            };
        });
    }
    *last_change_tick = world.change_tick();
}

/// A run condition for [`event_update_system`].
///
/// If [`signal_event_update_system`] has been run at least once,
/// we will wait for it to be run again before updating the events.
///
/// Otherwise, we will always update the events.
pub fn event_update_condition(maybe_signal: Option<Res<EventRegistry>>) -> bool {
    match maybe_signal {
        Some(signal) => match signal.should_update {
            ShouldUpdateEvents::Always | ShouldUpdateEvents::Ready => true,
            ShouldUpdateEvents::Waiting => false,
        },
        None => true,
    }
}

/// [`Iterator`] over sent [`EventIds`](`EventId`) from a batch.
pub struct SendBatchIds<E> {
    last_count: usize,
    event_count: usize,
    _marker: PhantomData<E>,
}

impl<E: Event> Iterator for SendBatchIds<E> {
    type Item = EventId<E>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.last_count >= self.event_count {
            return None;
        }

        let result = Some(EventId {
            id: self.last_count,
            _marker: PhantomData,
        });

        self.last_count += 1;

        result
    }
}

impl<E: Event> ExactSizeIterator for SendBatchIds<E> {
    fn len(&self) -> usize {
        self.event_count.saturating_sub(self.last_count)
    }
}

#[cfg(test)]
mod tests {
    use crate::system::assert_is_read_only_system;

    use super::*;

    #[derive(Event, Copy, Clone, PartialEq, Eq, Debug)]
    struct TestEvent {
        i: usize,
    }

    #[test]
    fn test_events() {
        let mut events = Events::<TestEvent>::default();
        let event_0 = TestEvent { i: 0 };
        let event_1 = TestEvent { i: 1 };
        let event_2 = TestEvent { i: 2 };

        // this reader will miss event_0 and event_1 because it wont read them over the course of
        // two updates
        let mut reader_missed = events.get_reader();

        let mut reader_a = events.get_reader();

        events.send(event_0);

        assert_eq!(
            get_events(&events, &mut reader_a),
            vec![event_0],
            "reader_a created before event receives event"
        );
        assert_eq!(
            get_events(&events, &mut reader_a),
            vec![],
            "second iteration of reader_a created before event results in zero events"
        );

        let mut reader_b = events.get_reader();

        assert_eq!(
            get_events(&events, &mut reader_b),
            vec![event_0],
            "reader_b created after event receives event"
        );
        assert_eq!(
            get_events(&events, &mut reader_b),
            vec![],
            "second iteration of reader_b created after event results in zero events"
        );

        events.send(event_1);

        let mut reader_c = events.get_reader();

        assert_eq!(
            get_events(&events, &mut reader_c),
            vec![event_0, event_1],
            "reader_c created after two events receives both events"
        );
        assert_eq!(
            get_events(&events, &mut reader_c),
            vec![],
            "second iteration of reader_c created after two event results in zero events"
        );

        assert_eq!(
            get_events(&events, &mut reader_a),
            vec![event_1],
            "reader_a receives next unread event"
        );

        events.update();

        let mut reader_d = events.get_reader();

        events.send(event_2);

        assert_eq!(
            get_events(&events, &mut reader_a),
            vec![event_2],
            "reader_a receives event created after update"
        );
        assert_eq!(
            get_events(&events, &mut reader_b),
            vec![event_1, event_2],
            "reader_b receives events created before and after update"
        );
        assert_eq!(
            get_events(&events, &mut reader_d),
            vec![event_0, event_1, event_2],
            "reader_d receives all events created before and after update"
        );

        events.update();

        assert_eq!(
            get_events(&events, &mut reader_missed),
            vec![event_2],
            "reader_missed missed events unread after two update() calls"
        );
    }

    fn get_events<E: Event + Clone>(
        events: &Events<E>,
        reader: &mut ManualEventReader<E>,
    ) -> Vec<E> {
        reader.read(events).cloned().collect::<Vec<E>>()
    }

    #[derive(Event, PartialEq, Eq, Debug)]
    struct E(usize);

    fn events_clear_and_read_impl(clear_func: impl FnOnce(&mut Events<E>)) {
        let mut events = Events::<E>::default();
        let mut reader = events.get_reader();

        assert!(reader.read(&events).next().is_none());

        events.send(E(0));
        assert_eq!(*reader.read(&events).next().unwrap(), E(0));
        assert_eq!(reader.read(&events).next(), None);

        events.send(E(1));
        clear_func(&mut events);
        assert!(reader.read(&events).next().is_none());

        events.send(E(2));
        events.update();
        events.send(E(3));

        assert!(reader.read(&events).eq([E(2), E(3)].iter()));
    }

    #[test]
    fn test_events_clear_and_read() {
        events_clear_and_read_impl(|events| events.clear());
    }

    #[test]
    fn test_events_drain_and_read() {
        events_clear_and_read_impl(|events| {
            assert!(events.drain().eq(vec![E(0), E(1)].into_iter()));
        });
    }

    #[test]
    fn test_events_extend_impl() {
        let mut events = Events::<TestEvent>::default();
        let mut reader = events.get_reader();

        events.extend(vec![TestEvent { i: 0 }, TestEvent { i: 1 }]);
        assert!(reader
            .read(&events)
            .eq([TestEvent { i: 0 }, TestEvent { i: 1 }].iter()));
    }

    #[test]
    fn test_events_empty() {
        let mut events = Events::<TestEvent>::default();
        assert!(events.is_empty());

        events.send(TestEvent { i: 0 });
        assert!(!events.is_empty());

        events.update();
        assert!(!events.is_empty());

        // events are only empty after the second call to update
        // due to double buffering.
        events.update();
        assert!(events.is_empty());
    }

    #[test]
    fn test_event_reader_len_empty() {
        let events = Events::<TestEvent>::default();
        assert_eq!(events.get_reader().len(&events), 0);
        assert!(events.get_reader().is_empty(&events));
    }

    #[test]
    fn test_event_reader_len_filled() {
        let mut events = Events::<TestEvent>::default();
        events.send(TestEvent { i: 0 });
        assert_eq!(events.get_reader().len(&events), 1);
        assert!(!events.get_reader().is_empty(&events));
    }

    #[test]
    fn test_event_iter_len_updated() {
        let mut events = Events::<TestEvent>::default();
        events.send(TestEvent { i: 0 });
        events.send(TestEvent { i: 1 });
        events.send(TestEvent { i: 2 });
        let mut reader = events.get_reader();
        let mut iter = reader.read(&events);
        assert_eq!(iter.len(), 3);
        iter.next();
        assert_eq!(iter.len(), 2);
        iter.next();
        assert_eq!(iter.len(), 1);
        iter.next();
        assert_eq!(iter.len(), 0);
    }

    #[test]
    fn test_event_reader_len_current() {
        let mut events = Events::<TestEvent>::default();
        events.send(TestEvent { i: 0 });
        let reader = events.get_reader_current();
        dbg!(&reader);
        dbg!(&events);
        assert!(reader.is_empty(&events));
        events.send(TestEvent { i: 0 });
        assert_eq!(reader.len(&events), 1);
        assert!(!reader.is_empty(&events));
    }

    #[test]
    fn test_event_reader_len_update() {
        let mut events = Events::<TestEvent>::default();
        events.send(TestEvent { i: 0 });
        events.send(TestEvent { i: 0 });
        let reader = events.get_reader();
        assert_eq!(reader.len(&events), 2);
        events.update();
        events.send(TestEvent { i: 0 });
        assert_eq!(reader.len(&events), 3);
        events.update();
        assert_eq!(reader.len(&events), 1);
        events.update();
        assert!(reader.is_empty(&events));
    }

    #[test]
    fn test_event_reader_clear() {
        use bevy_ecs::prelude::*;

        let mut world = World::new();
        let mut events = Events::<TestEvent>::default();
        events.send(TestEvent { i: 0 });
        world.insert_resource(events);

        let mut reader = IntoSystem::into_system(|mut events: EventReader<TestEvent>| -> bool {
            if !events.is_empty() {
                events.clear();
                false
            } else {
                true
            }
        });
        reader.initialize(&mut world);

        let is_empty = reader.run((), &mut world);
        assert!(!is_empty, "EventReader should not be empty");
        let is_empty = reader.run((), &mut world);
        assert!(is_empty, "EventReader should be empty");
    }

    #[test]
    fn test_update_drain() {
        let mut events = Events::<TestEvent>::default();
        let mut reader = events.get_reader();

        events.send(TestEvent { i: 0 });
        events.send(TestEvent { i: 1 });
        assert_eq!(reader.read(&events).count(), 2);

        let mut old_events = Vec::from_iter(events.update_drain());
        assert!(old_events.is_empty());

        events.send(TestEvent { i: 2 });
        assert_eq!(reader.read(&events).count(), 1);

        old_events.extend(events.update_drain());
        assert_eq!(old_events.len(), 2);

        old_events.extend(events.update_drain());
        assert_eq!(
            old_events,
            &[TestEvent { i: 0 }, TestEvent { i: 1 }, TestEvent { i: 2 }]
        );
    }

    #[allow(clippy::iter_nth_zero)]
    #[test]
    fn test_event_iter_nth() {
        use bevy_ecs::prelude::*;

        let mut world = World::new();
        world.init_resource::<Events<TestEvent>>();

        world.send_event(TestEvent { i: 0 });
        world.send_event(TestEvent { i: 1 });
        world.send_event(TestEvent { i: 2 });
        world.send_event(TestEvent { i: 3 });
        world.send_event(TestEvent { i: 4 });

        let mut schedule = Schedule::default();
        schedule.add_systems(|mut events: EventReader<TestEvent>| {
            let mut iter = events.read();

            assert_eq!(iter.next(), Some(&TestEvent { i: 0 }));
            assert_eq!(iter.nth(2), Some(&TestEvent { i: 3 }));
            assert_eq!(iter.nth(1), None);

            assert!(events.is_empty());
        });
        schedule.run(&mut world);
    }

    #[test]
    fn test_event_iter_last() {
        use bevy_ecs::prelude::*;

        let mut world = World::new();
        world.init_resource::<Events<TestEvent>>();

        let mut reader =
            IntoSystem::into_system(|mut events: EventReader<TestEvent>| -> Option<TestEvent> {
                events.read().last().copied()
            });
        reader.initialize(&mut world);

        let last = reader.run((), &mut world);
        assert!(last.is_none(), "EventReader should be empty");

        world.send_event(TestEvent { i: 0 });
        let last = reader.run((), &mut world);
        assert_eq!(last, Some(TestEvent { i: 0 }));

        world.send_event(TestEvent { i: 1 });
        world.send_event(TestEvent { i: 2 });
        world.send_event(TestEvent { i: 3 });
        let last = reader.run((), &mut world);
        assert_eq!(last, Some(TestEvent { i: 3 }));

        let last = reader.run((), &mut world);
        assert!(last.is_none(), "EventReader should be empty");
    }

    #[derive(Event, Clone, PartialEq, Debug, Default)]
    struct EmptyTestEvent;

    #[test]
    fn test_firing_empty_event() {
        let mut events = Events::<EmptyTestEvent>::default();
        events.send_default();

        let mut reader = events.get_reader();
        assert_eq!(get_events(&events, &mut reader), vec![EmptyTestEvent]);
    }

    #[test]
    fn ensure_reader_readonly() {
        fn reader_system(_: EventReader<EmptyTestEvent>) {}

        assert_is_read_only_system(reader_system);
    }

    #[test]
    fn test_send_events_ids() {
        let mut events = Events::<TestEvent>::default();
        let event_0 = TestEvent { i: 0 };
        let event_1 = TestEvent { i: 1 };
        let event_2 = TestEvent { i: 2 };

        let event_0_id = events.send(event_0);

        assert_eq!(
            events.get_event(event_0_id.id),
            Some((&event_0, event_0_id)),
            "Getting a sent event by ID should return the original event"
        );

        let mut event_ids = events.send_batch([event_1, event_2]);

        let event_id = event_ids.next().expect("Event 1 must have been sent");

        assert_eq!(
            events.get_event(event_id.id),
            Some((&event_1, event_id)),
            "Getting a sent event by ID should return the original event"
        );

        let event_id = event_ids.next().expect("Event 2 must have been sent");

        assert_eq!(
            events.get_event(event_id.id),
            Some((&event_2, event_id)),
            "Getting a sent event by ID should return the original event"
        );

        assert!(
            event_ids.next().is_none(),
            "Only sent two events; got more than two IDs"
        );
    }

    #[cfg(feature = "multi_threaded")]
    #[test]
    fn test_events_par_iter() {
        use crate::prelude::*;
        use std::sync::atomic::{AtomicUsize, Ordering};

        #[derive(Resource)]
        struct Counter(AtomicUsize);

        let mut world = World::new();
        world.init_resource::<Events<TestEvent>>();
        for _ in 0..100 {
            world.send_event(TestEvent { i: 1 });
        }
        let mut schedule = Schedule::default();
        schedule.add_systems(
            |mut events: EventReader<TestEvent>, counter: ResMut<Counter>| {
                events.par_read().for_each(|event| {
                    counter.0.fetch_add(event.i, Ordering::Relaxed);
                });
            },
        );
        world.insert_resource(Counter(AtomicUsize::new(0)));
        schedule.run(&mut world);
        let counter = world.remove_resource::<Counter>().unwrap();
        assert_eq!(counter.0.into_inner(), 100);

        world.insert_resource(Counter(AtomicUsize::new(0)));
        schedule.run(&mut world);
        let counter = world.remove_resource::<Counter>().unwrap();
        assert_eq!(counter.0.into_inner(), 0);
    }

    #[test]
    fn iter_current_update_events_iterates_over_current_events() {
        #[derive(Event, Clone)]
        struct TestEvent;

        let mut test_events = Events::<TestEvent>::default();

        // Starting empty
        assert_eq!(test_events.len(), 0);
        assert_eq!(test_events.iter_current_update_events().count(), 0);
        test_events.update();

        // Sending one event
        test_events.send(TestEvent);

        assert_eq!(test_events.len(), 1);
        assert_eq!(test_events.iter_current_update_events().count(), 1);
        test_events.update();

        // Sending two events on the next frame
        test_events.send(TestEvent);
        test_events.send(TestEvent);

        assert_eq!(test_events.len(), 3); // Events are double-buffered, so we see 1 + 2 = 3
        assert_eq!(test_events.iter_current_update_events().count(), 2);
        test_events.update();

        // Sending zero events
        assert_eq!(test_events.len(), 2); // Events are double-buffered, so we see 2 + 0 = 2
        assert_eq!(test_events.iter_current_update_events().count(), 0);
    }

    #[test]
    fn test_event_registry_can_add_and_remove_events_to_world() {
        use bevy_ecs::prelude::*;

        let mut world = World::new();
        EventRegistry::register_event::<TestEvent>(&mut world);

        let has_events = world.get_resource::<Events<TestEvent>>().is_some();

        assert!(has_events, "Should have the events resource");

        EventRegistry::deregister_events::<TestEvent>(&mut world);

        let has_events = world.get_resource::<Events<TestEvent>>().is_some();

        assert!(!has_events, "Should not have the events resource");
    }
}
