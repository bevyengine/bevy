#[cfg(feature = "multi_threaded")]
use crate::batching::BatchingStrategy;
use crate::message::{Message, MessageCursor, MessageId, MessageInstance, Messages};
use core::{iter::Chain, slice::IterMut};

/// An iterator that yields any unread messages from an [`MessageMutator`] or [`MessageCursor`].
///
/// [`MessageMutator`]: super::MessageMutator
#[derive(Debug)]
pub struct MessageMutIterator<'a, E: Message> {
    iter: MessageMutIteratorWithId<'a, E>,
}

impl<'a, E: Message> Iterator for MessageMutIterator<'a, E> {
    type Item = &'a mut E;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(message, _)| message)
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
        self.iter.last().map(|(message, _)| message)
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.iter.nth(n).map(|(message, _)| message)
    }
}

impl<'a, E: Message> ExactSizeIterator for MessageMutIterator<'a, E> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

/// An iterator that yields any unread messages (and their IDs) from an [`MessageMutator`] or [`MessageCursor`].
///
/// [`MessageMutator`]: super::MessageMutator
#[derive(Debug)]
pub struct MessageMutIteratorWithId<'a, E: Message> {
    mutator: &'a mut MessageCursor<E>,
    chain: Chain<IterMut<'a, MessageInstance<E>>, IterMut<'a, MessageInstance<E>>>,
    unread: usize,
}

impl<'a, E: Message> MessageMutIteratorWithId<'a, E> {
    /// Creates a new iterator that yields any `messages` that have not yet been seen by `mutator`.
    pub fn new(mutator: &'a mut MessageCursor<E>, messages: &'a mut Messages<E>) -> Self {
        let a_index = mutator
            .last_message_count
            .saturating_sub(messages.messages_a.start_message_count);
        let b_index = mutator
            .last_message_count
            .saturating_sub(messages.messages_b.start_message_count);
        let a = messages.messages_a.get_mut(a_index..).unwrap_or_default();
        let b = messages.messages_b.get_mut(b_index..).unwrap_or_default();

        let unread_count = a.len() + b.len();

        mutator.last_message_count = messages.message_count - unread_count;
        // Iterate the oldest first, then the newer messages
        let chain = a.iter_mut().chain(b.iter_mut());

        Self {
            mutator,
            chain,
            unread: unread_count,
        }
    }

    /// Iterate over only the messages.
    pub fn without_id(self) -> MessageMutIterator<'a, E> {
        MessageMutIterator { iter: self }
    }
}

impl<'a, E: Message> Iterator for MessageMutIteratorWithId<'a, E> {
    type Item = (&'a mut E, MessageId<E>);
    fn next(&mut self) -> Option<Self::Item> {
        match self
            .chain
            .next()
            .map(|instance| (&mut instance.message, instance.message_id))
        {
            Some(item) => {
                #[cfg(feature = "detailed_trace")]
                tracing::trace!("MessageMutator::iter() -> {}", item.1);
                self.mutator.last_message_count += 1;
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
        self.mutator.last_message_count += self.unread;
        self.unread
    }

    fn last(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        let MessageInstance {
            message_id,
            message,
        } = self.chain.last()?;
        self.mutator.last_message_count += self.unread;
        Some((message, *message_id))
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        if let Some(MessageInstance {
            message_id,
            message,
        }) = self.chain.nth(n)
        {
            self.mutator.last_message_count += n + 1;
            self.unread -= n + 1;
            Some((message, *message_id))
        } else {
            self.mutator.last_message_count += self.unread;
            self.unread = 0;
            None
        }
    }
}

impl<'a, E: Message> ExactSizeIterator for MessageMutIteratorWithId<'a, E> {
    fn len(&self) -> usize {
        self.unread
    }
}

/// A parallel iterator over `Message`s.
#[derive(Debug)]
#[cfg(feature = "multi_threaded")]
pub struct MessageMutParIter<'a, E: Message> {
    mutator: &'a mut MessageCursor<E>,
    slices: [&'a mut [MessageInstance<E>]; 2],
    batching_strategy: BatchingStrategy,
    #[cfg(not(target_arch = "wasm32"))]
    unread: usize,
}

#[cfg(feature = "multi_threaded")]
impl<'a, E: Message> MessageMutParIter<'a, E> {
    /// Creates a new parallel iterator over `messages` that have not yet been seen by `mutator`.
    pub fn new(mutator: &'a mut MessageCursor<E>, messages: &'a mut Messages<E>) -> Self {
        let a_index = mutator
            .last_message_count
            .saturating_sub(messages.messages_a.start_message_count);
        let b_index = mutator
            .last_message_count
            .saturating_sub(messages.messages_b.start_message_count);
        let a = messages.messages_a.get_mut(a_index..).unwrap_or_default();
        let b = messages.messages_b.get_mut(b_index..).unwrap_or_default();

        let unread_count = a.len() + b.len();
        mutator.last_message_count = messages.message_count - unread_count;

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

    /// Runs the provided closure for each unread message in parallel.
    ///
    /// Unlike normal iteration, the message order is not guaranteed in any form.
    ///
    /// # Panics
    /// If the [`ComputeTaskPool`] is not initialized. If using this from a message reader that is being
    /// initialized and run from the ECS scheduler, this should never panic.
    ///
    /// [`ComputeTaskPool`]: bevy_tasks::ComputeTaskPool
    pub fn for_each<FN: Fn(&'a mut E) + Send + Sync + Clone>(self, func: FN) {
        self.for_each_with_id(move |e, _| func(e));
    }

    /// Runs the provided closure for each unread message in parallel, like [`for_each`](Self::for_each),
    /// but additionally provides the `MessageId` to the closure.
    ///
    /// Note that the order of iteration is not guaranteed, but `MessageId`s are ordered by send order.
    ///
    /// # Panics
    /// If the [`ComputeTaskPool`] is not initialized. If using this from a message reader that is being
    /// initialized and run from the ECS scheduler, this should never panic.
    ///
    /// [`ComputeTaskPool`]: bevy_tasks::ComputeTaskPool
    #[cfg_attr(
        target_arch = "wasm32",
        expect(unused_mut, reason = "not mutated on this target")
    )]
    pub fn for_each_with_id<FN: Fn(&'a mut E, MessageId<E>) + Send + Sync + Clone>(
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
                        for message_instance in batch {
                            func(&mut message_instance.message, message_instance.message_id);
                        }
                    });
                }
            });

            // Messages are guaranteed to be read at this point.
            self.mutator.last_message_count += self.unread;
            self.unread = 0;
        }
    }

    /// Returns the number of [`Message`]s to be iterated.
    pub fn len(&self) -> usize {
        self.slices.iter().map(|s| s.len()).sum()
    }

    /// Returns [`true`] if there are no messages remaining in this iterator.
    pub fn is_empty(&self) -> bool {
        self.slices.iter().all(|x| x.is_empty())
    }
}

#[cfg(feature = "multi_threaded")]
impl<'a, E: Message> IntoIterator for MessageMutParIter<'a, E> {
    type IntoIter = MessageMutIteratorWithId<'a, E>;
    type Item = <Self::IntoIter as Iterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        let MessageMutParIter {
            mutator: reader,
            slices: [a, b],
            ..
        } = self;
        let unread = a.len() + b.len();
        let chain = a.iter_mut().chain(b);
        MessageMutIteratorWithId {
            mutator: reader,
            chain,
            unread,
        }
    }
}
