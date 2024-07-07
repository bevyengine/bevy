use crate as bevy_ecs;
use bevy_ecs::{
    event::{Event, EventId, EventInstance, ManualEventReader},
    system::Resource,
};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use bevy_utils::detailed_trace;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

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
///     before those updates.
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
    pub(crate) events_a: EventSequence<E>,
    /// Holds the newer events.
    pub(crate) events_b: EventSequence<E>,
    pub(crate) event_count: usize,
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
pub(crate) struct EventSequence<E: Event> {
    pub(crate) events: Vec<EventInstance<E>>,
    pub(crate) start_event_count: usize,
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
    use crate::{self as bevy_ecs, event::Events};
    use bevy_ecs_macros::Event;

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
}
