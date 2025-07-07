use bevy_ecs::event::{
    BufferedEvent, EventIterator, EventIteratorWithId, EventMutIterator, EventMutIteratorWithId,
    Events,
};
#[cfg(feature = "multi_threaded")]
use bevy_ecs::event::{EventMutParIter, EventParIter};
use core::marker::PhantomData;

/// Stores the state for an [`EventReader`] or [`EventMutator`].
///
/// Access to the [`Events<E>`] resource is required to read any incoming events.
///
/// In almost all cases, you should just use an [`EventReader`] or [`EventMutator`],
/// which will automatically manage the state for you.
///
/// However, this type can be useful if you need to manually track events,
/// such as when you're attempting to send and receive events of the same type in the same system.
///
/// # Example
///
/// ```
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::event::{BufferedEvent, Events, EventCursor};
///
/// #[derive(Event, BufferedEvent, Clone, Debug)]
/// struct MyEvent;
///
/// /// A system that both sends and receives events using a [`Local`] [`EventCursor`].
/// fn send_and_receive_events(
///     // The `Local` `SystemParam` stores state inside the system itself, rather than in the world.
///     // `EventCursor<T>` is the internal state of `EventMutator<T>`, which tracks which events have been seen.
///     mut local_event_reader: Local<EventCursor<MyEvent>>,
///     // We can access the `Events` resource mutably, allowing us to both read and write its contents.
///     mut events: ResMut<Events<MyEvent>>,
/// ) {
///     // We must collect the events to resend, because we can't mutate events while we're iterating over the events.
///     let mut events_to_resend = Vec::new();
///
///     for event in local_event_reader.read(&mut events) {
///          events_to_resend.push(event.clone());
///     }
///
///     for event in events_to_resend {
///         events.send(MyEvent);
///     }
/// }
///
/// # bevy_ecs::system::assert_is_system(send_and_receive_events);
/// ```
///
/// [`EventReader`]: super::EventReader
/// [`EventMutator`]: super::EventMutator
#[derive(Debug)]
pub struct EventCursor<E: BufferedEvent> {
    pub(super) last_event_count: usize,
    pub(super) _marker: PhantomData<E>,
}

impl<E: BufferedEvent> Default for EventCursor<E> {
    fn default() -> Self {
        EventCursor {
            last_event_count: 0,
            _marker: Default::default(),
        }
    }
}

impl<E: BufferedEvent> Clone for EventCursor<E> {
    fn clone(&self) -> Self {
        EventCursor {
            last_event_count: self.last_event_count,
            _marker: PhantomData,
        }
    }
}

impl<E: BufferedEvent> EventCursor<E> {
    /// See [`EventReader::read`](super::EventReader::read)
    pub fn read<'a>(&'a mut self, events: &'a Events<E>) -> EventIterator<'a, E> {
        self.read_with_id(events).without_id()
    }

    /// See [`EventMutator::read`](super::EventMutator::read)
    pub fn read_mut<'a>(&'a mut self, events: &'a mut Events<E>) -> EventMutIterator<'a, E> {
        self.read_mut_with_id(events).without_id()
    }

    /// See [`EventReader::read_with_id`](super::EventReader::read_with_id)
    pub fn read_with_id<'a>(&'a mut self, events: &'a Events<E>) -> EventIteratorWithId<'a, E> {
        EventIteratorWithId::new(self, events)
    }

    /// See [`EventMutator::read_with_id`](super::EventMutator::read_with_id)
    pub fn read_mut_with_id<'a>(
        &'a mut self,
        events: &'a mut Events<E>,
    ) -> EventMutIteratorWithId<'a, E> {
        EventMutIteratorWithId::new(self, events)
    }

    /// See [`EventReader::par_read`](super::EventReader::par_read)
    #[cfg(feature = "multi_threaded")]
    pub fn par_read<'a>(&'a mut self, events: &'a Events<E>) -> EventParIter<'a, E> {
        EventParIter::new(self, events)
    }

    /// See [`EventMutator::par_read`](super::EventMutator::par_read)
    #[cfg(feature = "multi_threaded")]
    pub fn par_read_mut<'a>(&'a mut self, events: &'a mut Events<E>) -> EventMutParIter<'a, E> {
        EventMutParIter::new(self, events)
    }

    /// See [`EventReader::len`](super::EventReader::len)
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

    /// See [`EventReader::is_empty()`](super::EventReader::is_empty)
    pub fn is_empty(&self, events: &Events<E>) -> bool {
        self.len(events) == 0
    }

    /// See [`EventReader::clear()`](super::EventReader::clear)
    pub fn clear(&mut self, events: &Events<E>) {
        self.last_event_count = events.event_count;
    }
}
