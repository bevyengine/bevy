use bevy_ecs::ResMut;
use bevy_utils::tracing::trace;
use std::{fmt, marker::PhantomData};

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

/// An event collection that represents the events that occurred within the last two [Events::update] calls. Events can be cheaply read using
/// an [EventReader]. This collection is meant to be paired with a system that calls [Events::update] exactly once per update/frame. [Events::update_system]
/// is a system that does this. [EventReader]s are expected to read events from this collection at least once per update/frame. If events are not handled
/// within one frame/update, they will be dropped.
///
/// # Example
/// ```
/// use bevy_app::Events;
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
/// [Events] is implemented using a double buffer. Each call to [Events::update] swaps buffers and clears out the oldest buffer.
/// [EventReader]s that read at least once per update will never drop events. [EventReader]s that read once within two updates might
/// still receive some events. [EventReader]s that read after two updates are guaranteed to drop all events that occurred before those updates.
///
/// The buffers in [Events] will grow indefinitely if [Events::update] is never called.
///
/// An alternative call pattern would be to call [Events::update] manually across frames to control when events are cleared. However
/// this complicates consumption
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
pub struct EventReader<T> {
    last_event_count: usize,
    _marker: PhantomData<T>,
}

impl<T> Default for EventReader<T> {
    fn default() -> Self {
        Self {
            last_event_count: 0,
            _marker: PhantomData::default(),
        }
    }
}

impl<T> EventReader<T> {
    /// Iterates over the events this EventReader has not seen yet. This updates the EventReader's
    /// event counter, which means subsequent event reads will not include events that happened before now.
    pub fn iter<'a>(&mut self, events: &'a Events<T>) -> impl DoubleEndedIterator<Item = &'a T> {
        self.iter_with_id(events).map(|(event, _id)| event)
    }

    /// Like [`iter`](Self::iter), except also returning the [`EventId`] of the events.
    pub fn iter_with_id<'a>(
        &mut self,
        events: &'a Events<T>,
    ) -> impl DoubleEndedIterator<Item = (&'a T, EventId<T>)> {
        self.iter_internal(events).map(|(event, id)| {
            trace!("EventReader::iter() -> {}", id);
            (event, id)
        })
    }

    /// Like [`iter_with_id`](Self::iter_with_id) except not emitting any traces for read messages.
    fn iter_internal<'a>(
        &mut self,
        events: &'a Events<T>,
    ) -> impl DoubleEndedIterator<Item = (&'a T, EventId<T>)> {
        // if the reader has seen some of the events in a buffer, find the proper index offset.
        // otherwise read all events in the buffer
        let a_index = if self.last_event_count > events.a_start_event_count {
            self.last_event_count - events.a_start_event_count
        } else {
            0
        };
        let b_index = if self.last_event_count > events.b_start_event_count {
            self.last_event_count - events.b_start_event_count
        } else {
            0
        };
        self.last_event_count = events.event_count;
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

    /// Retrieves the latest event that this EventReader hasn't seen yet. This updates the EventReader's
    /// event counter, which means subsequent event reads will not include events that happened before now.
    pub fn latest<'a>(&mut self, events: &'a Events<T>) -> Option<&'a T> {
        self.latest_with_id(events).map(|(event, _)| event)
    }

    /// Like [`latest`](Self::latest), except also returning the [`EventId`] of the event.
    pub fn latest_with_id<'a>(&mut self, events: &'a Events<T>) -> Option<(&'a T, EventId<T>)> {
        self.iter_internal(events).rev().next().map(|(event, id)| {
            trace!("EventReader::latest() -> {}", id);
            (event, id)
        })
    }

    /// Retrieves the latest event that matches the given `predicate` that this reader hasn't seen yet. This updates the EventReader's
    /// event counter, which means subsequent event reads will not include events that happened before now.
    pub fn find_latest<'a>(
        &mut self,
        events: &'a Events<T>,
        predicate: impl FnMut(&&T) -> bool,
    ) -> Option<&'a T> {
        self.find_latest_with_id(events, predicate)
            .map(|(event, _)| event)
    }

    /// Like [`find_latest`](Self::find_latest), except also returning the [`EventId`] of the event.
    pub fn find_latest_with_id<'a>(
        &mut self,
        events: &'a Events<T>,
        mut predicate: impl FnMut(&&T) -> bool,
    ) -> Option<(&'a T, EventId<T>)> {
        self.iter_internal(events)
            .rev()
            .find(|(event, _id)| predicate(event))
            .map(|(event, id)| {
                trace!("EventReader::find_latest() -> {}", id);
                (event, id)
            })
    }

    /// Retrieves the earliest event in `events` that this reader hasn't seen yet. This updates the EventReader's
    /// event counter, which means subsequent event reads will not include events that happened before now.
    pub fn earliest<'a>(&mut self, events: &'a Events<T>) -> Option<&'a T> {
        self.earliest_with_id(events).map(|(event, _)| event)
    }

    /// Like [`earliest`](Self::earliest), except also returning the [`EventId`] of the event.
    pub fn earliest_with_id<'a>(&mut self, events: &'a Events<T>) -> Option<(&'a T, EventId<T>)> {
        self.iter_internal(events).next().map(|(event, id)| {
            trace!("EventReader::earliest() -> {}", id);
            (event, id)
        })
    }
}

impl<T: bevy_ecs::Resource> Events<T> {
    /// "Sends" an `event` by writing it to the current event buffer. [EventReader]s can then read the event.
    pub fn send(&mut self, event: T) {
        let event_id = EventId {
            id: self.event_count,
            _marker: PhantomData,
        };
        trace!("Events::send() -> {}", event_id);

        let event_instance = EventInstance { event, event_id };

        match self.state {
            State::A => self.events_a.push(event_instance),
            State::B => self.events_b.push(event_instance),
        }

        self.event_count += 1;
    }

    /// Gets a new [EventReader]. This will include all events already in the event buffers.
    pub fn get_reader(&self) -> EventReader<T> {
        EventReader {
            last_event_count: 0,
            _marker: PhantomData,
        }
    }

    /// Gets a new [EventReader]. This will ignore all events already in the event buffers. It will read all future events.
    pub fn get_reader_current(&self) -> EventReader<T> {
        EventReader {
            last_event_count: self.event_count,
            _marker: PhantomData,
        }
    }

    /// Swaps the event buffers and clears the oldest event buffer. In general, this should be called once per frame/update.
    pub fn update(&mut self) {
        match self.state {
            State::A => {
                self.events_b = Vec::new();
                self.state = State::B;
                self.b_start_event_count = self.event_count;
            }
            State::B => {
                self.events_a = Vec::new();
                self.state = State::A;
                self.a_start_event_count = self.event_count;
            }
        }
    }

    /// A system that calls [Events::update] once per frame.
    pub fn update_system(mut events: ResMut<Self>) {
        events.update();
    }

    /// Removes all events.
    pub fn clear(&mut self) {
        self.events_a.clear();
        self.events_b.clear();
    }

    /// Creates a draining iterator that removes all events.
    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
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

    pub fn extend<I>(&mut self, events: I)
    where
        I: Iterator<Item = T>,
    {
        for event in events {
            self.send(event);
        }
    }

    /// Iterates over events that happened since the last "update" call.
    /// WARNING: You probably don't want to use this call. In most cases you should use an `EventReader`. You should only use
    /// this if you know you only need to consume events between the last `update()` call and your call to `iter_current_update_events`.
    /// If events happen outside that window, they will not be handled. For example, any events that happen after this call and before
    /// the next `update()` call will be dropped.
    pub fn iter_current_update_events(&self) -> impl DoubleEndedIterator<Item = &T> {
        match self.state {
            State::A => self.events_a.iter().map(map_instance_event),
            State::B => self.events_b.iter().map(map_instance_event),
        }
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

        // this reader will miss event_0 and event_1 because it wont read them over the course of two updates
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
            "reader_missed missed events unread after to update() calls"
        );
    }

    fn get_events(
        events: &Events<TestEvent>,
        reader: &mut EventReader<TestEvent>,
    ) -> Vec<TestEvent> {
        reader.iter(events).cloned().collect::<Vec<TestEvent>>()
    }
}
