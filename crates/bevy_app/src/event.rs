use bevy_ecs::{
    component::Component,
    system::{Local, Res, ResMut, SystemParam},
};
use bevy_utils::tracing::trace;
use std::cmp::min;
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
/// [`AppBuilder::add_event`]. [EventReader]s are expected to read events from this collection at
/// least once per loop/frame.  
/// Events will persist across a single frame boundary and so ordering of event producers and
/// consumers is not critical (although poorly-planned ordering may cause accumulating lag).
/// If events are not handled by the end of the frame after they are updated, they will be
/// dropped silently.
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
/// but can be done by adding your event as a resource instead of using [`AppBuilder::add_event`].
#[derive(Debug)]
pub struct Events<T> {
    buffer: Vec<EventInstance<T>>,
    event_count: usize,
    event_offset: usize,
}

impl<T> Default for Events<T> {
    fn default() -> Self {
        Events {
            buffer: Vec::new(),
            event_count: 0,
            event_offset: 0,
        }
    }
}

fn map_instance_event_with_id<T>(event_instance: &EventInstance<T>) -> (&T, EventId<T>) {
    (&event_instance.event, event_instance.event_id)
}

fn map_instance_event<T>(event_instance: &EventInstance<T>) -> &T {
    &event_instance.event
}

pub struct EventSubscriptions<T: Component> {
    last_event_counts: Vec<usize>,
    last_event_count: usize,
    _phantom: PhantomData<T>,
}

impl<T: Component> Default for EventSubscriptions<T> {
    fn default() -> Self {
        EventSubscriptions::<T> {
            last_event_counts: Vec::new(),
            last_event_count: 0,
            _phantom: Default::default(),
        }
    }
}

/// Reads events of type `T` in order and tracks which events have already been read.
#[derive(SystemParam)]
pub struct EventReader<'a, T: Component> {
    subscription_id: Local<'a, (Option<usize>, PhantomData<T>)>,
    event_subscriptions: ResMut<'a, EventSubscriptions<T>>,
    events: Res<'a, Events<T>>,
}

/// Sends events of type `T`.
#[derive(SystemParam)]
pub struct EventWriter<'a, T: Component> {
    events: ResMut<'a, Events<T>>,
}

impl<'a, T: Component> EventWriter<'a, T> {
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
    let index = if *last_event_count > events.event_offset {
        *last_event_count - events.event_offset
    } else {
        0
    };
    *last_event_count = events.event_count;
    events
        .buffer
        .get(index..)
        .unwrap_or_else(|| &[])
        .iter()
        .map(map_instance_event_with_id)
}

impl<'a, T: Component> EventReader<'a, T> {
    /// Iterates over the events this EventReader has not seen yet. This updates the EventReader's
    /// event counter, which means subsequent event reads will not include events that happened
    /// before now.
    pub fn iter(&mut self) -> impl DoubleEndedIterator<Item = &T> {
        self.iter_with_id().map(|(event, _id)| event)
    }

    fn get_subscription_id(
        subscription_id: &mut Option<usize>,
        event_subscriptions: &mut EventSubscriptions<T>,
        event_count: usize,
    ) -> usize {
        *subscription_id.get_or_insert_with(|| {
            event_subscriptions.last_event_counts.push(event_count);
            event_subscriptions.last_event_counts.len() - 1
        })
    }

    /// Like [`iter`](Self::iter), except also returning the [`EventId`] of the events.
    pub fn iter_with_id(&mut self) -> impl DoubleEndedIterator<Item = (&T, EventId<T>)> {
        let subscription_id = Self::get_subscription_id(
            &mut self.subscription_id.0,
            &mut *self.event_subscriptions,
            self.events.event_offset,
        );
        let mut last_event_count = self
            .event_subscriptions
            .last_event_counts
            .get_mut(subscription_id)
            .unwrap();
        internal_event_reader(&mut last_event_count, &self.events).map(|(event, id)| {
            trace!("EventReader::iter() -> {}", id);
            (event, id)
        })
    }
}

impl<T: Component> Events<T> {
    /// "Sends" an `event` by writing it to the current event buffer. [EventReader]s can then read
    /// the event.
    pub fn send(&mut self, event: T) {
        let event_id = EventId {
            id: self.event_count,
            _marker: PhantomData,
        };
        trace!("Events::send() -> {}", event_id);

        let event_instance = EventInstance { event_id, event };

        self.buffer.push(event_instance);

        self.event_count += 1;
    }

    /// Gets a new [ManualEventReader]. This will include all events already in the event buffers.
    pub fn get_reader(&mut self) -> ManualEventReader<T> {
        ManualEventReader {
            last_event_count: 0,
            _marker: PhantomData,
        }
    }

    /// Gets a new [ManualEventReader]. This will ignore all events already in the event buffers. It
    /// will read all future events.
    pub fn get_reader_current(&mut self) -> ManualEventReader<T> {
        ManualEventReader {
            last_event_count: self.event_count,
            _marker: PhantomData,
        }
    }

    /// Swaps the event buffers and clears the oldest event buffer. In general, this should be
    /// called once per frame/update.
    pub fn update(&mut self, event_subscriptions: &mut EventSubscriptions<T>) {
        if event_subscriptions.last_event_counts.is_empty() {
            event_subscriptions.last_event_count = self.event_count;
            self.event_offset = self.event_count;
            self.buffer.clear();
        } else {
            let read_count = event_subscriptions
                .last_event_counts
                .iter()
                .fold(usize::max_value(), |count, next| min(count, *next));
            let remove_index = read_count - self.event_offset;
            self.event_offset = read_count;
            self.buffer.drain(0..remove_index);
            event_subscriptions.last_event_count = self.event_count;
        }
    }

    /// A system that calls [Events::update] once per frame.
    pub fn update_system(
        mut events: ResMut<Self>,
        mut event_subscriptions: ResMut<EventSubscriptions<T>>,
    ) {
        events.update(&mut *event_subscriptions);
    }

    /// Removes all events.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.event_offset = self.event_count;
    }

    /// Creates a draining iterator that removes all events.
    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.event_offset = self.event_count;
        let map = |i: EventInstance<T>| i.event;
        self.buffer.drain(..).map(map)
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
    /// WARNING: You probably don't want to use this call. In most cases you should use an
    /// `EventReader`. You should only use this if you know you only need to consume events
    /// between the last `update()` call and your call to `iter_current_update_events`.
    /// If events happen outside that window, they will not be handled. For example, any events that
    /// happen after this call and before the next `update()` call will be dropped.
    pub fn iter_current_update_events(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.buffer.iter().map(map_instance_event)
    }
}

impl<T: Component> EventSubscriptions<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::schedule::{Schedule, SystemStage};
    use bevy_ecs::system::IntoSystem;
    use bevy_ecs::world::World;

    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    struct TestEvent {
        i: usize,
    }

    #[test]
    fn test_events() {
        fn event_writer_system1(mut event_writer: EventWriter<TestEvent>) {
            let event_0 = TestEvent { i: 0 };
            let event_1 = TestEvent { i: 1 };
            event_writer.send(event_0);
            event_writer.send(event_1);
        }
        fn event_writer_system2(mut event_writer: EventWriter<TestEvent>) {
            let event_2 = TestEvent { i: 2 };
            event_writer.send(event_2);
        }
        fn event_reader_system1(mut event_reader: EventReader<TestEvent>) {
            for _event in event_reader.iter() {
                // hi
            }
        }
        let mut schedule = Schedule::default();
        let update1 = SystemStage::single(event_writer_system1.system());
        let update2 = SystemStage::single(event_reader_system1.system());
        let update3 = SystemStage::single(event_writer_system2.system());
        let update4 = SystemStage::single(event_reader_system1.system());
        let update5 = SystemStage::single(Events::<TestEvent>::update_system.system());
        schedule.add_stage("update1", update1);
        schedule.add_stage("update2", update2);
        schedule.add_stage("update3", update3);
        schedule.add_stage("update4", update4);
        schedule.add_stage("update5", update5);

        let mut world = World::default();
        world.insert_resource(Events::<TestEvent>::default());
        world.insert_resource(EventSubscriptions::<TestEvent>::default());
        schedule.run_once(&mut world);
        let events = world.get_resource::<Events<TestEvent>>().unwrap();
        assert_eq!(
            events.event_offset, 2,
            "All subscribed systems read the first two events."
        );

        schedule.run_once(&mut world);
        let events = world.get_resource::<Events<TestEvent>>().unwrap();
        assert_eq!(
            events.event_offset, 5,
            "All subscribed systems read all events from last frame plus 2 new events from this frame"
        );
    }

    #[test]
    fn test_manual_events() {
        let mut events = Events::<TestEvent>::default();
        let mut event_subscriptions = EventSubscriptions::<TestEvent>::default();
        let event_0 = TestEvent { i: 0 };
        let event_1 = TestEvent { i: 1 };
        let event_2 = TestEvent { i: 2 };

        let mut reader_slow = events.get_reader();
        let mut reader_a = events.get_reader();

        events.send(event_0);

        assert_eq!(
            get_events(&mut events, &mut reader_a),
            vec![event_0],
            "reader_a created before event receives event"
        );
        assert_eq!(
            get_events(&mut events, &mut reader_a),
            vec![],
            "second iteration of reader_a created before event results in zero events"
        );

        let mut reader_b = events.get_reader();

        assert_eq!(
            get_events(&mut events, &mut reader_b),
            vec![event_0],
            "reader_b created after event receives event"
        );
        assert_eq!(
            get_events(&mut events, &mut reader_b),
            vec![],
            "second iteration of reader_b created after event results in zero events"
        );

        events.send(event_1);

        let mut reader_c = events.get_reader();

        assert_eq!(
            get_events(&mut events, &mut reader_c),
            vec![event_0, event_1],
            "reader_c created after two events receives both events"
        );
        assert_eq!(
            get_events(&mut events, &mut reader_c),
            vec![],
            "second iteration of reader_c created after two event results in zero events"
        );

        assert_eq!(
            get_events(&mut events, &mut reader_a),
            vec![event_1],
            "reader_a receives next unread event"
        );

        events.update(&mut event_subscriptions);

        let mut reader_d = events.get_reader();

        events.send(event_2);

        assert_eq!(
            get_events(&mut events, &mut reader_a),
            vec![event_2],
            "reader_a receives event created after update"
        );
        assert_eq!(
            get_events(&mut events, &mut reader_b),
            vec![event_2],
            "reader_b receives events created after update"
        );
        assert_eq!(
            get_events(&mut events, &mut reader_c),
            vec![event_2],
            "reader_c receives event created since last fetch"
        );
        assert_eq!(
            get_events(&mut events, &mut reader_d),
            vec![event_2],
            "reader_d receives all events created after update"
        );

        events.update(&mut event_subscriptions);

        assert_eq!(
            get_events(&mut events, &mut reader_slow),
            vec![],
            "reader_slow misses all events"
        );

        events.update(&mut event_subscriptions);

        // At this point, the event buffer should be emptied and the count and offset
        // should be the same since all subscribed readers have read all the events.
        assert_eq!(
            events.event_count, events.event_offset,
            "all subscribed readers have read all events"
        );
        assert_eq!(events.buffer.is_empty(), true, "event buffer is empty");
    }

    fn get_events(
        events: &mut Events<TestEvent>,
        reader: &mut ManualEventReader<TestEvent>,
    ) -> Vec<TestEvent> {
        reader.iter(events).cloned().collect::<Vec<TestEvent>>()
    }
}
