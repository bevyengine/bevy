#[cfg(feature = "multi_threaded")]
use bevy_ecs::batching::BatchingStrategy;
use bevy_ecs::event::{BufferedEvent, EventCursor, EventId, EventInstance, Events};
use core::{iter::Chain, slice::IterMut};

/// An iterator that yields any unread events from an [`EventMutator`] or [`EventCursor`].
///
/// [`EventMutator`]: super::EventMutator
#[derive(Debug)]
pub struct EventMutIterator<'a, E: BufferedEvent> {
    iter: EventMutIteratorWithId<'a, E>,
}

impl<'a, E: BufferedEvent> Iterator for EventMutIterator<'a, E> {
    type Item = &'a mut E;
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

impl<'a, E: BufferedEvent> ExactSizeIterator for EventMutIterator<'a, E> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

/// An iterator that yields any unread events (and their IDs) from an [`EventMutator`] or [`EventCursor`].
///
/// [`EventMutator`]: super::EventMutator
#[derive(Debug)]
pub struct EventMutIteratorWithId<'a, E: BufferedEvent> {
    mutator: &'a mut EventCursor<E>,
    chain: Chain<IterMut<'a, EventInstance<E>>, IterMut<'a, EventInstance<E>>>,
    unread: usize,
}

impl<'a, E: BufferedEvent> EventMutIteratorWithId<'a, E> {
    /// Creates a new iterator that yields any `events` that have not yet been seen by `mutator`.
    pub fn new(mutator: &'a mut EventCursor<E>, events: &'a mut Events<E>) -> Self {
        let a_index = mutator
            .last_event_count
            .saturating_sub(events.events_a.start_event_count);
        let b_index = mutator
            .last_event_count
            .saturating_sub(events.events_b.start_event_count);
        let a = events.events_a.get_mut(a_index..).unwrap_or_default();
        let b = events.events_b.get_mut(b_index..).unwrap_or_default();

        let unread_count = a.len() + b.len();

        mutator.last_event_count = events.event_count - unread_count;
        // Iterate the oldest first, then the newer events
        let chain = a.iter_mut().chain(b.iter_mut());

        Self {
            mutator,
            chain,
            unread: unread_count,
        }
    }

    /// Iterate over only the events.
    pub fn without_id(self) -> EventMutIterator<'a, E> {
        EventMutIterator { iter: self }
    }
}

impl<'a, E: BufferedEvent> Iterator for EventMutIteratorWithId<'a, E> {
    type Item = (&'a mut E, EventId<E>);
    fn next(&mut self) -> Option<Self::Item> {
        match self
            .chain
            .next()
            .map(|instance| (&mut instance.event, instance.event_id))
        {
            Some(item) => {
                #[cfg(feature = "detailed_trace")]
                tracing::trace!("EventMutator::iter() -> {}", item.1);
                self.mutator.last_event_count += 1;
                self.unread -= 1;
                Some(item)
            }
            None => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.chain.size_hint()
    }

    fn count(self) -> usize {
        self.mutator.last_event_count += self.unread;
        self.unread
    }

    fn last(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        let EventInstance { event_id, event } = self.chain.last()?;
        self.mutator.last_event_count += self.unread;
        Some((event, *event_id))
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        if let Some(EventInstance { event_id, event }) = self.chain.nth(n) {
            self.mutator.last_event_count += n + 1;
            self.unread -= n + 1;
            Some((event, *event_id))
        } else {
            self.mutator.last_event_count += self.unread;
            self.unread = 0;
            None
        }
    }
}

impl<'a, E: BufferedEvent> ExactSizeIterator for EventMutIteratorWithId<'a, E> {
    fn len(&self) -> usize {
        self.unread
    }
}

/// A parallel iterator over `BufferedEvent`s.
#[derive(Debug)]
#[cfg(feature = "multi_threaded")]
pub struct EventMutParIter<'a, E: BufferedEvent> {
    mutator: &'a mut EventCursor<E>,
    slices: [&'a mut [EventInstance<E>]; 2],
    batching_strategy: BatchingStrategy,
    #[cfg(not(target_arch = "wasm32"))]
    unread: usize,
}

#[cfg(feature = "multi_threaded")]
impl<'a, E: BufferedEvent> EventMutParIter<'a, E> {
    /// Creates a new parallel iterator over `events` that have not yet been seen by `mutator`.
    pub fn new(mutator: &'a mut EventCursor<E>, events: &'a mut Events<E>) -> Self {
        let a_index = mutator
            .last_event_count
            .saturating_sub(events.events_a.start_event_count);
        let b_index = mutator
            .last_event_count
            .saturating_sub(events.events_b.start_event_count);
        let a = events.events_a.get_mut(a_index..).unwrap_or_default();
        let b = events.events_b.get_mut(b_index..).unwrap_or_default();

        let unread_count = a.len() + b.len();
        mutator.last_event_count = events.event_count - unread_count;

        Self {
            mutator,
            slices: [a, b],
            batching_strategy: BatchingStrategy::default(),
            #[cfg(not(target_arch = "wasm32"))]
            unread: unread_count,
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
    pub fn for_each<FN: Fn(&'a mut E) + Send + Sync + Clone>(self, func: FN) {
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
    #[cfg_attr(
        target_arch = "wasm32",
        expect(unused_mut, reason = "not mutated on this target")
    )]
    pub fn for_each_with_id<FN: Fn(&'a mut E, EventId<E>) + Send + Sync + Clone>(
        mut self,
        func: FN,
    ) {
        #[cfg(target_arch = "wasm32")]
        {
            self.into_iter().for_each(|(e, i)| func(e, i));
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let pool = bevy_tasks::ComputeTaskPool::get();
            let thread_count = pool.thread_num();
            if thread_count <= 1 {
                return self.into_iter().for_each(|(e, i)| func(e, i));
            }

            let batch_size = self
                .batching_strategy
                .calc_batch_size(|| self.len(), thread_count);
            let chunks = self.slices.map(|s| s.chunks_mut(batch_size));

            pool.scope(|scope| {
                for batch in chunks.into_iter().flatten() {
                    let func = func.clone();
                    scope.spawn(async move {
                        for event in batch {
                            func(&mut event.event, event.event_id);
                        }
                    });
                }
            });

            // Events are guaranteed to be read at this point.
            self.mutator.last_event_count += self.unread;
            self.unread = 0;
        }
    }

    /// Returns the number of [`BufferedEvent`]s to be iterated.
    pub fn len(&self) -> usize {
        self.slices.iter().map(|s| s.len()).sum()
    }

    /// Returns [`true`] if there are no events remaining in this iterator.
    pub fn is_empty(&self) -> bool {
        self.slices.iter().all(|x| x.is_empty())
    }
}

#[cfg(feature = "multi_threaded")]
impl<'a, E: BufferedEvent> IntoIterator for EventMutParIter<'a, E> {
    type IntoIter = EventMutIteratorWithId<'a, E>;
    type Item = <Self::IntoIter as Iterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        let EventMutParIter {
            mutator: reader,
            slices: [a, b],
            ..
        } = self;
        let unread = a.len() + b.len();
        let chain = a.iter_mut().chain(b);
        EventMutIteratorWithId {
            mutator: reader,
            chain,
            unread,
        }
    }
}
