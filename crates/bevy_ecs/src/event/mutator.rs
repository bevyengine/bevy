#[cfg(feature = "multi_threaded")]
use bevy_ecs::event::EventMutParIter;
use bevy_ecs::{
    event::{BufferedEvent, EventCursor, EventMutIterator, EventMutIteratorWithId, Events},
    system::{Local, ResMut, SystemParam},
};

/// Mutably reads events of type `T` keeping track of which events have already been read
/// by each system allowing multiple systems to read the same events. Ideal for chains of systems
/// that all want to modify the same events.
///
/// # Usage
///
/// `EventMutators`s are usually declared as a [`SystemParam`].
/// ```
/// # use bevy_ecs::prelude::*;
///
/// #[derive(BufferedEvent, Debug)]
/// pub struct MyEvent(pub u32); // Custom event type.
/// fn my_system(mut reader: EventMutator<MyEvent>) {
///     for event in reader.read() {
///         event.0 += 1;
///         println!("received event: {:?}", event);
///     }
/// }
/// ```
///
/// # Concurrency
///
/// Multiple systems with `EventMutator<T>` of the same event type can not run concurrently.
/// They also can not be executed in parallel with [`EventReader`] or [`EventWriter`].
///
/// # Clearing, Reading, and Peeking
///
/// Events are stored in a double buffered queue that switches each frame. This switch also clears the previous
/// frame's events. Events should be read each frame otherwise they may be lost. For manual control over this
/// behavior, see [`Events`].
///
/// Most of the time systems will want to use [`EventMutator::read()`]. This function creates an iterator over
/// all events that haven't been read yet by this system, marking the event as read in the process.
///
/// [`EventReader`]: super::EventReader
/// [`EventWriter`]: super::EventWriter
#[derive(SystemParam, Debug)]
pub struct EventMutator<'w, 's, E: BufferedEvent> {
    pub(super) reader: Local<'s, EventCursor<E>>,
    #[system_param(validation_message = "BufferedEvent not initialized")]
    events: ResMut<'w, Events<E>>,
}

impl<'w, 's, E: BufferedEvent> EventMutator<'w, 's, E> {
    /// Iterates over the events this [`EventMutator`] has not seen yet. This updates the
    /// [`EventMutator`]'s event counter, which means subsequent event reads will not include events
    /// that happened before now.
    pub fn read(&mut self) -> EventMutIterator<'_, E> {
        self.reader.read_mut(&mut self.events)
    }

    /// Like [`read`](Self::read), except also returning the [`EventId`](super::EventId) of the events.
    pub fn read_with_id(&mut self) -> EventMutIteratorWithId<'_, E> {
        self.reader.read_mut_with_id(&mut self.events)
    }

    /// Returns a parallel iterator over the events this [`EventMutator`] has not seen yet.
    /// See also [`for_each`](super::EventParIter::for_each).
    ///
    /// # Example
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use std::sync::atomic::{AtomicUsize, Ordering};
    ///
    /// #[derive(BufferedEvent)]
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
    /// schedule.add_systems(|mut events: EventMutator<MyEvent>, counter: Res<Counter>| {
    ///     events.par_read().for_each(|MyEvent { value }| {
    ///         counter.0.fetch_add(*value, Ordering::Relaxed);
    ///     });
    /// });
    /// for value in 0..100 {
    ///     world.write_event(MyEvent { value });
    /// }
    /// schedule.run(&mut world);
    /// let Counter(counter) = world.remove_resource::<Counter>().unwrap();
    /// // all events were processed
    /// assert_eq!(counter.into_inner(), 4950);
    /// ```
    #[cfg(feature = "multi_threaded")]
    pub fn par_read(&mut self) -> EventMutParIter<'_, E> {
        self.reader.par_read_mut(&mut self.events)
    }

    /// Determines the number of events available to be read from this [`EventMutator`] without consuming any.
    pub fn len(&self) -> usize {
        self.reader.len(&self.events)
    }

    /// Returns `true` if there are no events available to read.
    ///
    /// # Example
    ///
    /// The following example shows a useful pattern where some behavior is triggered if new events are available.
    /// [`EventMutator::clear()`] is used so the same events don't re-trigger the behavior the next time the system runs.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// #[derive(BufferedEvent)]
    /// struct CollisionEvent;
    ///
    /// fn play_collision_sound(mut events: EventMutator<CollisionEvent>) {
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
    /// This means these events will not appear in calls to [`EventMutator::read()`] or
    /// [`EventMutator::read_with_id()`] and [`EventMutator::is_empty()`] will return `true`.
    ///
    /// For usage, see [`EventMutator::is_empty()`].
    pub fn clear(&mut self) {
        self.reader.clear(&self.events);
    }
}
