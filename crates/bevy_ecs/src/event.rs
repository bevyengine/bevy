//! Event handling types.

use crate::system::{Local, Res, ResMut, SystemParam};
use crate::{self as bevy_ecs, system::Resource};
use bevy_utils::tracing::trace;
use std::{
    fmt::{self},
    hash::Hash,
    marker::PhantomData,
};

/// An `EventId` uniquely identifies an event.
///
/// An `EventId` can among other things be used to trace the flow of an event from the point it was
/// sent to the point it was processed.
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EventId<T> {
    pub id: usize,
    _marker: PhantomData<T>,
}

impl<T> Copy for EventId<T> {}
impl<T> Clone for EventId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> fmt::Display for EventId<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

impl<T> fmt::Debug for EventId<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "event<{}>#{}",
            std::any::type_name::<T>().split("::").last().unwrap(),
            self.id,
        )
    }
}

#[derive(Debug)]
struct EventInstance<T> {
    pub event_id: EventId<T>,
    pub event: T,
}

#[derive(Debug)]
enum State {
    A,
    B,
}

/// An event collection that represents the events that occurred within the last two
/// [`Events::update`] calls.
/// Events can be written to using an [`EventWriter`]
/// and are typically cheaply read using an [`EventReader`].
///
/// Each event can be consumed by multiple systems, in parallel,
/// with consumption tracked by the [`EventReader`] on a per-system basis.
///
/// This collection is meant to be paired with a system that calls
/// [`Events::update`] exactly once per update/frame.
///
/// [`Events::update_system`] is a system that does this, typically intialized automatically using
/// [`App::add_event`]. [EventReader]s are expected to read events from this collection at
/// least once per loop/frame.
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
/// [Events] is implemented using a double buffer. Each call to [Events::update] swaps buffers and
/// clears out the oldest buffer. [EventReader]s that read at least once per update will never drop
/// events. [EventReader]s that read once within two updates might still receive some events.
/// [EventReader]s that read after two updates are guaranteed to drop all events that occurred
/// before those updates.
///
/// The buffers in [Events] will grow indefinitely if [Events::update] is never called.
///
/// An alternative call pattern would be to call [Events::update] manually across frames to control
/// when events are cleared.
/// This complicates consumption and risks ever-expanding memory usage if not cleaned up,
/// but can be done by adding your event as a resource instead of using [`App::add_event`].
///
/// [`App::add_event`]: https://docs.rs/bevy/*/bevy/app/struct.App.html#method.add_event
#[derive(Debug)]
pub struct Events<T> {
    events_a: Vec<EventInstance<T>>,
    events_b: Vec<EventInstance<T>>,
    a_start_event_count: usize,
    b_start_event_count: usize,
    event_count: usize,
    state: State,
}

impl<T> Default for Events<T> {
    fn default() -> Self {
        Events {
            a_start_event_count: 0,
            b_start_event_count: 0,
            event_count: 0,
            events_a: Vec::new(),
            events_b: Vec::new(),
            state: State::A,
        }
    }
}

fn map_instance_event_with_id<T>(event_instance: &EventInstance<T>) -> (&T, EventId<T>) {
    (&event_instance.event, event_instance.event_id)
}

fn map_instance_event<T>(event_instance: &EventInstance<T>) -> &T {
    &event_instance.event
}

/// Reads events of type `T` in order and tracks which events have already been read.
#[derive(SystemParam)]
pub struct EventReader<'w, 's, T: Resource> {
    last_event_count: Local<'s, (usize, PhantomData<T>)>,
    events: Res<'w, Events<T>>,
}

/// Sends events of type `T`.
#[derive(SystemParam)]
pub struct EventWriter<'w, 's, T: Resource> {
    events: ResMut<'w, Events<T>>,
    #[system_param(ignore)]
    marker: PhantomData<&'s usize>,
}

impl<'w, 's, T: Resource> EventWriter<'w, 's, T> {
    pub fn send(&mut self, event: T) {
        self.events.send(event);
    }

    pub fn send_batch(&mut self, events: impl Iterator<Item = T>) {
        self.events.extend(events);
    }
}

pub struct ManualEventReader<T> {
    last_event_count: usize,
    _marker: PhantomData<T>,
}

impl<T> Default for ManualEventReader<T> {
    fn default() -> Self {
        ManualEventReader {
            last_event_count: 0,
            _marker: Default::default(),
        }
    }
}

impl<T> ManualEventReader<T> {
    /// See [`EventReader::iter`]
    pub fn iter<'a>(&mut self, events: &'a Events<T>) -> impl DoubleEndedIterator<Item = &'a T> {
        internal_event_reader(&mut self.last_event_count, events).map(|(e, _)| e)
    }

    /// See [`EventReader::iter_with_id`]
    pub fn iter_with_id<'a>(
        &mut self,
        events: &'a Events<T>,
    ) -> impl DoubleEndedIterator<Item = (&'a T, EventId<T>)> {
        internal_event_reader(&mut self.last_event_count, events)
    }
}

/// Like [`iter_with_id`](EventReader::iter_with_id) except not emitting any traces for read
/// messages.
fn internal_event_reader<'a, T>(
    last_event_count: &mut usize,
    events: &'a Events<T>,
) -> impl DoubleEndedIterator<Item = (&'a T, EventId<T>)> {
    // if the reader has seen some of the events in a buffer, find the proper index offset.
    // otherwise read all events in the buffer
    let a_index = if *last_event_count > events.a_start_event_count {
        *last_event_count - events.a_start_event_count
    } else {
        0
    };
    let b_index = if *last_event_count > events.b_start_event_count {
        *last_event_count - events.b_start_event_count
    } else {
        0
    };
    *last_event_count = events.event_count;
    match events.state {
        State::A => events
            .events_b
            .get(b_index..)
            .unwrap_or_else(|| &[])
            .iter()
            .map(map_instance_event_with_id)
            .chain(
                events
                    .events_a
                    .get(a_index..)
                    .unwrap_or_else(|| &[])
                    .iter()
                    .map(map_instance_event_with_id),
            ),
        State::B => events
            .events_a
            .get(a_index..)
            .unwrap_or_else(|| &[])
            .iter()
            .map(map_instance_event_with_id)
            .chain(
                events
                    .events_b
                    .get(b_index..)
                    .unwrap_or_else(|| &[])
                    .iter()
                    .map(map_instance_event_with_id),
            ),
    }
}

impl<'w, 's, T: Resource> EventReader<'w, 's, T> {
    /// Iterates over the events this EventReader has not seen yet. This updates the EventReader's
    /// event counter, which means subsequent event reads will not include events that happened
    /// before now.
    pub fn iter(&mut self) -> impl DoubleEndedIterator<Item = &T> {
        self.iter_with_id().map(|(event, _id)| event)
    }

    /// Like [`iter`](Self::iter), except also returning the [`EventId`] of the events.
    pub fn iter_with_id(&mut self) -> impl DoubleEndedIterator<Item = (&T, EventId<T>)> {
        internal_event_reader(&mut self.last_event_count.0, &self.events).map(|(event, id)| {
            trace!("EventReader::iter() -> {}", id);
            (event, id)
        })
    }
}

impl<T: Resource> Events<T> {
    /// "Sends" an `event` by writing it to the current event buffer. [EventReader]s can then read
    /// the event.
    pub fn send(&mut self, event: T) {
        let event_id = EventId {
            id: self.event_count,
            _marker: PhantomData,
        };
        trace!("Events::send() -> id: {}", event_id);

        let event_instance = EventInstance { event_id, event };

        match self.state {
            State::A => self.events_a.push(event_instance),
            State::B => self.events_b.push(event_instance),
        }

        self.event_count += 1;
    }

    /// Gets a new [ManualEventReader]. This will include all events already in the event buffers.
    pub fn get_reader(&self) -> ManualEventReader<T> {
        ManualEventReader {
            last_event_count: 0,
            _marker: PhantomData,
        }
    }

    /// Gets a new [ManualEventReader]. This will ignore all events already in the event buffers. It
    /// will read all future events.
    pub fn get_reader_current(&self) -> ManualEventReader<T> {
        ManualEventReader {
            last_event_count: self.event_count,
            _marker: PhantomData,
        }
    }

    /// Swaps the event buffers and clears the oldest event buffer. In general, this should be
    /// called once per frame/update.
    pub fn update(&mut self) {
        match self.state {
            State::A => {
                self.events_b.clear();
                self.state = State::B;
                self.b_start_event_count = self.event_count;
            }
            State::B => {
                self.events_a.clear();
                self.state = State::A;
                self.a_start_event_count = self.event_count;
            }
        }
    }

    /// A system that calls [Events::update] once per frame.
    pub fn update_system(mut events: ResMut<Self>) {
        events.update();
    }

    #[inline]
    fn reset_start_event_count(&mut self) {
        self.a_start_event_count = self.event_count;
        self.b_start_event_count = self.event_count;
    }

    /// Removes all events.
    #[inline]
    pub fn clear(&mut self) {
        self.reset_start_event_count();
        self.events_a.clear();
        self.events_b.clear();
    }

    /// Returns true if there are no events in this collection.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.events_a.is_empty() && self.events_b.is_empty()
    }

    /// Creates a draining iterator that removes all events.
    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.reset_start_event_count();

        let map = |i: EventInstance<T>| i.event;
        match self.state {
            State::A => self
                .events_b
                .drain(..)
                .map(map)
                .chain(self.events_a.drain(..).map(map)),
            State::B => self
                .events_a
                .drain(..)
                .map(map)
                .chain(self.events_b.drain(..).map(map)),
        }
    }

    /// Iterates over events that happened since the last "update" call.
    /// WARNING: You probably don't want to use this call. In most cases you should use an
    /// `EventReader`. You should only use this if you know you only need to consume events
    /// between the last `update()` call and your call to `iter_current_update_events`.
    /// If events happen outside that window, they will not be handled. For example, any events that
    /// happen after this call and before the next `update()` call will be dropped.
    pub fn iter_current_update_events(&self) -> impl DoubleEndedIterator<Item = &T> {
        match self.state {
            State::A => self.events_a.iter().map(map_instance_event),
            State::B => self.events_b.iter().map(map_instance_event),
        }
    }
}

impl<T> std::iter::Extend<T> for Events<T> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
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

        match self.state {
            State::A => self.events_a.extend(events),
            State::B => self.events_b.extend(events),
        }

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

    fn get_events(
        events: &Events<TestEvent>,
        reader: &mut ManualEventReader<TestEvent>,
    ) -> Vec<TestEvent> {
        reader.iter(events).cloned().collect::<Vec<TestEvent>>()
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
}
