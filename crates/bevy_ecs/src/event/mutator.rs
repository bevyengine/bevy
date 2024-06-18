use crate as bevy_ecs;
use bevy_ecs::{
    event::{Event, EventMutatorIterator, EventMutatorIteratorWithId, Events},
    system::{Local, ResMut, SystemParam},
};
use std::marker::PhantomData;
#[cfg(feature = "multi_threaded")]
use bevy_ecs::event::EventMutatorParIter;

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
/// #[derive(Event, Debug)]
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
/// They also can not be executed in parallel with [`EventReader`] and [`EventWriter`].
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
#[derive(SystemParam, Debug)]
pub struct EventMutator<'w, 's, E: Event> {
    pub(super) reader: Local<'s, ManualEventMutator<E>>,
    events: ResMut<'w, Events<E>>,
}

impl<'w, 's, E: Event> EventMutator<'w, 's, E> {
    /// Iterates over the events this [`EventReader`] has not seen yet. This updates the
    /// [`EventReader`]'s event counter, which means subsequent event reads will not include events
    /// that happened before now.
    pub fn read(&mut self) -> EventMutatorIterator<'_, E> {
        self.reader.read(&mut self.events)
    }

    /// Like [`read`](Self::read), except also returning the [`EventId`] of the events.
    pub fn read_with_id(&mut self) -> EventMutatorIteratorWithId<'_, E> {
        self.reader.read_with_id(&mut self.events)
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
    pub fn par_read(&mut self) -> EventMutatorParIter<'_, E> {
        self.reader.par_read(&mut self.events)
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
pub struct ManualEventMutator<E: Event> {
    pub(super) last_event_count: usize,
    pub(super) _marker: PhantomData<E>,
}

impl<E: Event> Default for ManualEventMutator<E> {
    fn default() -> Self {
        ManualEventMutator {
            last_event_count: 0,
            _marker: Default::default(),
        }
    }
}

impl<E: Event> Clone for ManualEventMutator<E> {
    fn clone(&self) -> Self {
        ManualEventMutator {
            last_event_count: self.last_event_count,
            _marker: PhantomData,
        }
    }
}

#[allow(clippy::len_without_is_empty)] // Check fails since the is_empty implementation has a signature other than `(&self) -> bool`
impl<E: Event> ManualEventMutator<E> {
    /// See [`EventReader::read`]
    pub fn read<'a>(&'a mut self, events: &'a mut Events<E>) -> EventMutatorIterator<'a, E> {
        self.read_with_id(events).without_id()
    }

    /// See [`EventReader::read_with_id`]
    pub fn read_with_id<'a>(
        &'a mut self,
        events: &'a mut Events<E>,
    ) -> EventMutatorIteratorWithId<'a, E> {
        EventMutatorIteratorWithId::new(self, events)
    }

    /// See [`EventReader::par_read`]
    #[cfg(feature = "multi_threaded")]
    pub fn par_read<'a>(&'a mut self, events: &'a mut Events<E>) -> EventMutatorParIter<'a, E> {
        EventMutatorParIter::new(self, events)
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
