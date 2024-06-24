use crate as bevy_ecs;
#[cfg(feature = "multi_threaded")]
use bevy_ecs::event::EventPeekParIter;
#[cfg(feature = "multi_threaded")]
use bevy_ecs::event::EventReadParIter;
use bevy_ecs::{
    event::{
        Event, EventCursor, EventPeekIterator, EventPeekIteratorWithId, EventReadIterator,
        EventReadIteratorWithId, Events,
    },
    system::{Local, Res, SystemParam},
};

/// Reads events of type `T`, keeping track of which events have already been read
/// by each system allowing multiple systems to read the same events.
///
/// # Usage
///
/// `EventReader`s are usually declared as a [`SystemParam`].
/// ```
/// # use bevy_ecs::prelude::*;
///
/// #[derive(Event, Debug)]
/// pub struct MyEvent; // Custom event type.
/// fn my_system(mut reader: EventReader<MyEvent>) {
///     for event in reader.read() {
///         println!("received event: {:?}", event);
///     }
/// }
/// ```
///
/// # Concurrency
///
/// Multiple systems with `EventReader<T>` of the same event type can run concurrently. Systems with
/// `EventReader<T>` can not be executed in parallel with those with `EventWriter<T>`.
///
/// # Clearing, Reading, and Peeking
///
/// Events are stored in a double buffered queue that switches each frame. This switch also clears the previous
/// frame's events. Events should be read each frame otherwise they may be lost. For manual control over this
/// behavior, see [`Events`].
///
/// Most of the time systems will want to use [`EventReader::read()`]. This function creates an iterator over
/// all events that haven't been read yet by this system, marking the event as read in the process.
///
/// Occasionally, it may be useful to iterate over all events that haven't been read yet without marking
/// them as read. This can be accomplished with [`EventReader::peek()`].
///
#[derive(SystemParam, Debug)]
pub struct EventReader<'w, 's, E: Event> {
    pub(super) reader: Local<'s, EventCursor<E>>,
    events: Res<'w, Events<E>>,
}

impl<'w, 's, E: Event> EventReader<'w, 's, E> {
    /// Iterates over the events this [`EventReader`] has not seen yet. This updates the
    /// [`EventReader`]'s event counter, which means subsequent event reads will not include events
    /// that happened before now.
    pub fn read(&mut self) -> EventReadIterator<'_, E> {
        self.reader.read(&self.events)
    }

    /// Iterates over all the events this [`EventReader`] currently has, including those that have
    /// been read (see [`EventReader::read()`],[`EventReader::read_with_id()`], [`EventReader::par_read`]).
    /// Unlike [`read`](Self::read), this does not update the [`EventReader`]'s event counter and
    /// thus does not mark the event as read.
    pub fn peek(&self) -> EventPeekIterator<'_, E> {
        self.reader.peek(&self.events)
    }

    /// Like [`read`](Self::read), except also returning the [`EventId`] of the events.
    pub fn read_with_id(&mut self) -> EventReadIteratorWithId<'_, E> {
        self.reader.read_with_id(&self.events)
    }

    /// Like [`peek`](Self::peek), except also returning the [`EventId`] of the events.
    pub fn peek_with_id(&self) -> EventPeekIteratorWithId<'_, E> {
        self.reader.peek_with_id(&self.events)
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
    /// for _ in 0..100 {
    ///     world.send_event(MyEvent { value: 1 });
    /// }
    /// schedule.run(&mut world);
    /// let Counter(counter) = world.remove_resource::<Counter>().unwrap();
    /// // all events were processed
    /// assert_eq!(counter.into_inner(), 100);
    /// ```
    ///
    #[cfg(feature = "multi_threaded")]
    pub fn par_read(&mut self) -> EventReadParIter<'_, E> {
        self.reader.par_read(&self.events)
    }

    /// Returns a parallel iterator over the events this [`EventReader`] has not read yet.
    /// Unlike [`par_read`](Self::par_read) this does not update the [`EventReader`]'s
    /// event counter and thus does not mark the event as read.
    ///
    /// For more information on this see ['peek'](Self::read) and [`for_each`](EventRefParIter::for_each).
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
    ///     events.par_peek().for_each(|MyEvent { value }| {
    ///         counter.0.fetch_add(*value, Ordering::Relaxed);
    ///     });
    /// });
    /// for _ in 0..100 {
    ///     world.send_event(MyEvent { value: 1 });
    /// }
    /// schedule.run(&mut world);
    /// let Counter(counter) = world.remove_resource::<Counter>().unwrap();
    /// // all events were processed
    /// assert_eq!(counter.into_inner(), 100);
    /// ```
    ///
    #[cfg(feature = "multi_threaded")]
    pub fn par_peek(&self) -> EventPeekParIter<'_, E> {
        self.reader.par_peek(&self.events)
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
