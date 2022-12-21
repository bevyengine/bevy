//! Event handling types.

use crate as bevy_ecs;
use crate::system::{Local, Res, ResMut, Resource, SystemParam};
use bevy_utils::tracing::trace;
use std::ops::{Deref, DerefMut};
use std::{fmt, hash::Hash, marker::PhantomData};

/// A type that can be stored in an [`Events<E>`] resource
/// You can conveniently access events using the [`EventReader`] and [`EventWriter`] system parameter.
///
/// Events must be thread-safe.
pub trait Event: Send + Sync + 'static {}
impl<T> Event for T where T: Send + Sync + 'static {}

/// An `EventId` uniquely identifies an event.
///
/// An `EventId` can among other things be used to trace the flow of an event from the point it was
/// sent to the point it was processed.
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EventId<E: Event> {
    pub id: usize,
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

#[derive(Debug)]
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
/// [`Events::update_system`] is a system that does this, typically initialized automatically using
/// [`add_event`](https://docs.rs/bevy/*/bevy/app/struct.App.html#method.add_event).
/// [`EventReader`]s are expected to read events from this collection at least once per loop/frame.
/// Events will persist across a single frame boundary and so ordering of event producers and
/// consumers is not critical (although poorly-planned ordering may cause accumulating lag).
/// If events are not handled by the end of the frame after they are updated, they will be
/// dropped silently.
///
/// # Example
/// ```
/// use bevy_ecs::event::Events;
///
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
/// for event in reader.iter(&events) {
///     assert_eq!(event.value, 1)
/// }
///
/// // events are only processed once per reader
/// assert_eq!(reader.iter(&events).count(), 0);
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
pub struct Events<E: Event> {
    /// Holds the oldest still active events.
    /// Note that a.start_event_count + a.len() should always === events_b.start_event_count.
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
    pub fn oldest_event_count(&self) -> usize {
        self.events_a
            .start_event_count
            .min(self.events_b.start_event_count)
    }
}

#[derive(Debug)]
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
#[derive(SystemParam, Debug)]
pub struct EventReader<'w, 's, E: Event> {
    reader: Local<'s, ManualEventReader<E>>,
    events: Res<'w, Events<E>>,
}

impl<'w, 's, E: Event> EventReader<'w, 's, E> {
    /// Iterates over the events this [`EventReader`] has not seen yet. This updates the
    /// [`EventReader`]'s event counter, which means subsequent event reads will not include events
    /// that happened before now.
    pub fn iter(&mut self) -> impl DoubleEndedIterator<Item = &E> + ExactSizeIterator<Item = &E> {
        self.iter_with_id().map(|(event, _id)| event)
    }

    /// Like [`iter`](Self::iter), except also returning the [`EventId`] of the events.
    pub fn iter_with_id(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = (&E, EventId<E>)> + ExactSizeIterator<Item = (&E, EventId<E>)>
    {
        self.reader.iter_with_id(&self.events).inspect(|(_, id)| {
            trace!("EventReader::iter() -> {}", id);
        })
    }

    /// Determines the number of events available to be read from this [`EventReader`] without consuming any.
    pub fn len(&self) -> usize {
        self.reader.len(&self.events)
    }

    /// Determines if no events are available to be read without consuming any.
    /// If you need to consume the iterator you can use [`EventReader::clear`].
    ///
    /// # Example
    ///
    /// The following example shows a common pattern of this function in conjunction with `clear`
    /// to avoid leaking events to the next schedule iteration while also checking if it was emitted.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
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
        self.len() == 0
    }

    /// Consumes the iterator.
    ///
    /// This means all currently available events will be removed before the next frame.
    /// This is useful when multiple events are sent in a single frame and you want
    /// to react to one or more events without needing to know how many were sent.
    /// In those situations you generally want to consume those events to make sure they don't appear in the next frame.
    ///
    /// For more information see [`EventReader::is_empty()`].
    pub fn clear(&mut self) {
        self.iter().last();
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
/// pub struct MyEvent; // Custom event type.
/// fn my_system(mut writer: EventWriter<MyEvent>) {
///     writer.send(MyEvent);
/// }
///
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
///
/// # Limitations
///
/// `EventWriter` can only send events of one specific type, which must be known at compile-time.
/// This is not a problem most of the time, but you may find a situation where you cannot know
/// ahead of time every kind of event you'll need to send. In this case, you can use the "type-erased event" pattern.
///
/// ```
/// # use bevy_ecs::{prelude::*, event::Events};
///
/// # pub struct MyEvent;
/// fn send_untyped(mut commands: Commands) {
///     // Send an event of a specific type without having to declare that
///     // type as a SystemParam.
///     //
///     // Effectively, we're just moving the type parameter from the /type/ to the /method/,
///     // which allows one to do all kinds of clever things with type erasure, such as sending
///     // custom events to unknown 3rd party plugins (modding API).
///     //
///     // NOTE: the event won't actually be sent until commands get flushed
///     // at the end of the current stage.
///     commands.add(|w: &mut World| {
///         let mut events_resource = w.resource_mut::<Events<_>>();
///         events_resource.send(MyEvent);
///     });
/// }
/// ```
/// Note that this is considered *non-idiomatic*, and should only be used when `EventWriter` will not work.
#[derive(SystemParam)]
pub struct EventWriter<'w, E: Event> {
    events: ResMut<'w, Events<E>>,
}

impl<'w, E: Event> EventWriter<'w, E> {
    /// Sends an `event`. [`EventReader`]s can then read the event.
    /// See [`Events`] for details.
    pub fn send(&mut self, event: E) {
        self.events.send(event);
    }

    pub fn send_batch(&mut self, events: impl IntoIterator<Item = E>) {
        self.events.extend(events);
    }

    /// Sends the default value of the event. Useful when the event is an empty struct.
    pub fn send_default(&mut self)
    where
        E: Default,
    {
        self.events.send_default();
    }
}

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

#[allow(clippy::len_without_is_empty)] // Check fails since the is_empty implementation has a signature other than `(&self) -> bool`
impl<E: Event> ManualEventReader<E> {
    /// See [`EventReader::iter`]
    pub fn iter<'a>(
        &'a mut self,
        events: &'a Events<E>,
    ) -> impl DoubleEndedIterator<Item = &'a E> + ExactSizeIterator<Item = &'a E> {
        self.iter_with_id(events).map(|(e, _)| e)
    }

    /// See [`EventReader::iter_with_id`]
    pub fn iter_with_id<'a>(
        &'a mut self,
        events: &'a Events<E>,
    ) -> impl DoubleEndedIterator<Item = (&'a E, EventId<E>)>
           + ExactSizeIterator<Item = (&'a E, EventId<E>)> {
        let a_index = (self.last_event_count).saturating_sub(events.events_a.start_event_count);
        let b_index = (self.last_event_count).saturating_sub(events.events_b.start_event_count);
        let a = events.events_a.get(a_index..).unwrap_or_default();
        let b = events.events_b.get(b_index..).unwrap_or_default();

        let unread_count = a.len() + b.len();
        // Ensure `len` is implemented correctly
        debug_assert_eq!(unread_count, self.len(events));
        self.last_event_count = events.event_count - unread_count;
        // Iterate the oldest first, then the newer events
        let iterator = a.iter().chain(b.iter());
        iterator
            .map(|e| (&e.event, e.event_id))
            .with_exact_size(unread_count)
            .inspect(move |(_, id)| self.last_event_count = (id.id + 1).max(self.last_event_count))
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

    /// See [`EventReader::is_empty`]
    pub fn is_empty(&self, events: &Events<E>) -> bool {
        self.len(events) == 0
    }
}

trait IteratorExt {
    fn with_exact_size(self, len: usize) -> ExactSize<Self>
    where
        Self: Sized,
    {
        ExactSize::new(self, len)
    }
}
impl<I> IteratorExt for I where I: Iterator {}

#[must_use = "iterators are lazy and do nothing unless consumed"]
#[derive(Clone)]
struct ExactSize<I> {
    iter: I,
    len: usize,
}
impl<I> ExactSize<I> {
    fn new(iter: I, len: usize) -> Self {
        ExactSize { iter, len }
    }
}

impl<I: Iterator> Iterator for ExactSize<I> {
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        self.iter.next().map(|e| {
            self.len -= 1;
            e
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<I: DoubleEndedIterator> DoubleEndedIterator for ExactSize<I> {
    #[inline]
    fn next_back(&mut self) -> Option<I::Item> {
        self.iter.next_back().map(|e| {
            self.len -= 1;
            e
        })
    }
}
impl<I: Iterator> ExactSizeIterator for ExactSize<I> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<E: Event> Events<E> {
    /// "Sends" an `event` by writing it to the current event buffer. [`EventReader`]s can then read
    /// the event.
    pub fn send(&mut self, event: E) {
        let event_id = EventId {
            id: self.event_count,
            _marker: PhantomData,
        };
        trace!("Events::send() -> id: {}", event_id);

        let event_instance = EventInstance { event_id, event };

        self.events_b.push(event_instance);
        self.event_count += 1;
    }

    /// Sends the default value of the event. Useful when the event is an empty struct.
    pub fn send_default(&mut self)
    where
        E: Default,
    {
        self.send(Default::default());
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
    pub fn update(&mut self) {
        std::mem::swap(&mut self.events_a, &mut self.events_b);
        self.events_b.clear();
        self.events_b.start_event_count = self.event_count;
        debug_assert_eq!(
            self.events_a.start_event_count + self.events_a.len(),
            self.events_b.start_event_count
        );
    }

    /// A system that calls [`Events::update`] once per frame.
    pub fn update_system(mut events: ResMut<Self>) {
        events.update();
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

    #[inline]
    pub fn len(&self) -> usize {
        self.events_a.len() + self.events_b.len()
    }

    /// Returns true if there are no events in this collection.
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
    pub fn iter_current_update_events(
        &self,
    ) -> impl DoubleEndedIterator<Item = &E> + ExactSizeIterator<Item = &E> {
        self.events_b.iter().map(|i| &i.event)
    }
}

impl<E: Event> std::iter::Extend<E> for Events<E> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = E>,
    {
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

        trace!(
            "Events::extend() -> ids: ({}..{})",
            self.event_count,
            event_count
        );
        self.event_count = event_count;
    }
}

#[cfg(test)]
mod tests {
    use crate::{prelude::World, system::SystemState};

    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
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
        reader.iter(events).cloned().collect::<Vec<E>>()
    }

    #[derive(PartialEq, Eq, Debug)]
    struct E(usize);

    fn events_clear_and_read_impl(clear_func: impl FnOnce(&mut Events<E>)) {
        let mut events = Events::<E>::default();
        let mut reader = events.get_reader();

        assert!(reader.iter(&events).next().is_none());

        events.send(E(0));
        assert_eq!(*reader.iter(&events).next().unwrap(), E(0));
        assert_eq!(reader.iter(&events).next(), None);

        events.send(E(1));
        clear_func(&mut events);
        assert!(reader.iter(&events).next().is_none());

        events.send(E(2));
        events.update();
        events.send(E(3));

        assert!(reader.iter(&events).eq([E(2), E(3)].iter()));
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
            .iter(&events)
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
        let mut iter = reader.iter(&events);
        assert_eq!(iter.len(), 3);
        iter.next();
        assert_eq!(iter.len(), 2);
        iter.next_back();
        assert_eq!(iter.len(), 1);
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

    #[derive(Clone, PartialEq, Debug, Default)]
    struct EmptyTestEvent;

    #[test]
    fn test_firing_empty_event() {
        let mut events = Events::<EmptyTestEvent>::default();
        events.send_default();

        let mut reader = events.get_reader();
        assert_eq!(
            get_events(&events, &mut reader),
            vec![EmptyTestEvent::default()]
        );
    }

    #[test]
    fn ensure_reader_readonly() {
        fn read_for<E: Event>() {
            let mut world = World::new();
            world.init_resource::<Events<E>>();
            let mut state = SystemState::<EventReader<E>>::new(&mut world);
            // This can only work if EventReader only reads the world
            let _reader = state.get(&world);
        }
        read_for::<EmptyTestEvent>();
    }
}
