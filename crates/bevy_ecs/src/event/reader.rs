use crate as bevy_ecs;
#[cfg(feature = "multi_threaded")]
use bevy_ecs::event::EventParIter;
use bevy_ecs::{
    event::{Event, EventCursor, EventIterator, EventIteratorWithId, Events},
    system::{Local, Res, SystemParam},
};

/// Reads events of type `T` in order and tracks which events have already been read.
///
/// # Concurrency
///
/// Unlike [`EventWriter<T>`], systems with `EventReader<T>` param can be executed concurrently
/// (but not concurrently with `EventWriter<T>` or `EventMutator<T>` systems for the same event type).
///
/// [`EventWriter<T>`]: super::EventWriter
#[derive(SystemParam, Debug)]
pub struct EventReader<'w, 's, E: Event> {
    pub(super) reader: Local<'s, EventCursor<E>>,
    events: Res<'w, Events<E>>,
}

impl<'w, 's, E: Event> EventReader<'w, 's, E> {
    /// Iterates over the events this [`EventReader`] has not seen yet. This updates the
    /// [`EventReader`]'s event counter, which means subsequent event reads will not include events
    /// that happened before now.
    pub fn read(&mut self) -> EventIterator<'_, E> {
        self.reader.read(&self.events)
    }

    /// Like [`read`](Self::read), except also returning the [`EventId`](super::EventId) of the events.
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
