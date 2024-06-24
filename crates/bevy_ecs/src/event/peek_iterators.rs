use crate as bevy_ecs;
#[cfg(feature = "multi_threaded")]
use bevy_ecs::batching::BatchingStrategy;
use bevy_ecs::event::{Event, EventCursor, EventId, EventInstance, Events};
use bevy_utils::detailed_trace;
use std::{iter::Chain, slice::Iter};

/// An iterator that yields any unread events from an [`EventReader`] or [`EventCursor`].
#[derive(Debug)]
pub struct EventPeekIterator<'a, E: Event> {
    iter: EventPeekIteratorWithId<'a, E>,
}

impl<'a, E: Event> Iterator for EventPeekIterator<'a, E> {
    type Item = &'a E;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(event, _)| event)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn count(self) -> usize {
        self.iter.count()
    }

    fn last(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        self.iter.last().map(|(event, _)| event)
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.iter.nth(n).map(|(event, _)| event)
    }
}

impl<'a, E: Event> ExactSizeIterator for EventPeekIterator<'a, E> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

/// An iterator that yields references to any unread events (and their IDs) from an [`EventReader`] or [`EventCursor`].
#[derive(Debug)]
pub struct EventPeekIteratorWithId<'a, E: Event> {
    chain: Chain<Iter<'a, EventInstance<E>>, Iter<'a, EventInstance<E>>>,
    unread: usize,
}

impl<'a, E: Event> EventPeekIteratorWithId<'a, E> {
    /// Creates a new iterator that yields any `events` that have not yet been seen by `reader`.
    pub fn new(reader: &'a EventCursor<E>, events: &'a Events<E>) -> Self {
        let a_index = reader
            .last_event_count
            .saturating_sub(events.events_a.start_event_count);
        let b_index = reader
            .last_event_count
            .saturating_sub(events.events_b.start_event_count);
        let a = events.events_a.get(a_index..).unwrap_or_default();
        let b = events.events_b.get(b_index..).unwrap_or_default();

        let unread = a.len() + b.len();
        // Ensure `len` is implemented correctly
        debug_assert_eq!(unread, reader.len(events));

        // Iterate the oldest first, then the newer events
        let chain = a.iter().chain(b.iter());

        Self { chain, unread }
    }

    /// Iterate over the events only.
    pub fn without_id(self) -> EventPeekIterator<'a, E> {
        EventPeekIterator { iter: self }
    }
}

impl<'a, E: Event> Iterator for EventPeekIteratorWithId<'a, E> {
    type Item = (&'a E, EventId<E>);
    fn next(&mut self) -> Option<Self::Item> {
        match self
            .chain
            .next()
            .map(|instance| (&instance.event, instance.event_id))
        {
            Some(item) => {
                detailed_trace!("EventReader::iter() -> {}", item.1);
                Some(item)
            }
            None => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.chain.size_hint()
    }

    fn count(self) -> usize {
        self.unread
    }

    fn last(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        let EventInstance { event_id, event } = self.chain.last()?;
        Some((event, *event_id))
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        if let Some(EventInstance { event_id, event }) = self.chain.nth(n) {
            Some((event, *event_id))
        } else {
            None
        }
    }
}

impl<'a, E: Event> ExactSizeIterator for EventPeekIteratorWithId<'a, E> {
    fn len(&self) -> usize {
        self.unread
    }
}

/// A parallel iterator over `Event`s.
#[derive(Debug)]
#[cfg(feature = "multi_threaded")]
pub struct EventPeekParIter<'a, E: Event> {
    slices: [&'a [EventInstance<E>]; 2],
    batching_strategy: BatchingStrategy,
}

#[cfg(feature = "multi_threaded")]
impl<'a, E: Event> EventPeekParIter<'a, E> {
    /// Creates a new parallel iterator over `events` that have not yet been seen by `reader`.
    pub fn new(reader: &'a EventCursor<E>, events: &'a Events<E>) -> Self {
        let a_index = reader
            .last_event_count
            .saturating_sub(events.events_a.start_event_count);
        let b_index = reader
            .last_event_count
            .saturating_sub(events.events_b.start_event_count);
        let a = events.events_a.get(a_index..).unwrap_or_default();
        let b = events.events_b.get(b_index..).unwrap_or_default();

        Self {
            slices: [a, b],
            batching_strategy: BatchingStrategy::default(),
        }
    }

    /// Changes the batching strategy used when iterating.
    ///
    /// For more information on how this affects the resultant iteration, see
    /// [`BatchingStrategy`].
    pub fn batching_strategy(mut self, strategy: BatchingStrategy) -> Self {
        self.batching_strategy = strategy;
        self
    }

    /// Runs the provided closure for each unread event in parallel.
    ///
    /// Unlike normal iteration, the event order is not guaranteed in any form.
    ///
    /// # Panics
    /// If the [`ComputeTaskPool`] is not initialized. If using this from an event reader that is being
    /// initialized and run from the ECS scheduler, this should never panic.
    ///
    /// [`ComputeTaskPool`]: bevy_tasks::ComputeTaskPool
    pub fn for_each<FN: Fn(&'a E) + Send + Sync + Clone>(self, func: FN) {
        self.for_each_with_id(move |e, _| func(e));
    }

    /// Runs the provided closure for each unread event in parallel, like [`for_each`](Self::for_each),
    /// but additionally provides the `EventId` to the closure.
    ///
    /// Note that the order of iteration is not guaranteed, but `EventId`s are ordered by send order.
    ///
    /// # Panics
    /// If the [`ComputeTaskPool`] is not initialized. If using this from an event reader that is being
    /// initialized and run from the ECS scheduler, this should never panic.
    ///
    /// [`ComputeTaskPool`]: bevy_tasks::ComputeTaskPool
    pub fn for_each_with_id<FN: Fn(&'a E, EventId<E>) + Send + Sync + Clone>(self, func: FN) {
        #[cfg(any(target_arch = "wasm32", not(feature = "multi_threaded")))]
        {
            self.into_iter().for_each(|(e, i)| func(e, i));
        }

        #[cfg(all(not(target_arch = "wasm32"), feature = "multi_threaded"))]
        {
            let pool = bevy_tasks::ComputeTaskPool::get();
            let thread_count = pool.thread_num();
            if thread_count <= 1 {
                return self.into_iter().for_each(|(e, i)| func(e, i));
            }

            let batch_size = self
                .batching_strategy
                .calc_batch_size(|| self.len(), thread_count);
            let chunks = self.slices.map(|s| s.chunks_exact(batch_size));
            let remainders = chunks.each_ref().map(|c| c.remainder());

            pool.scope(|scope| {
                for batch in chunks.into_iter().flatten().chain(remainders) {
                    let func = func.clone();
                    scope.spawn(async move {
                        for event in batch {
                            func(&event.event, event.event_id);
                        }
                    });
                }
            });
        }
    }

    /// Returns the number of [`Event`]s to be iterated.
    pub fn len(&self) -> usize {
        self.slices.iter().map(|s| s.len()).sum()
    }

    /// Returns [`true`] if there are no events remaining in this iterator.
    pub fn is_empty(&self) -> bool {
        self.slices.iter().all(|x| x.is_empty())
    }
}

#[cfg(feature = "multi_threaded")]
impl<'a, E: Event> IntoIterator for EventPeekParIter<'a, E> {
    type IntoIter = EventPeekIteratorWithId<'a, E>;
    type Item = <Self::IntoIter as Iterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        let EventPeekParIter { slices: [a, b], .. } = self;
        let unread = a.len() + b.len();
        let chain = a.iter().chain(b);
        EventPeekIteratorWithId { chain, unread }
    }
}
