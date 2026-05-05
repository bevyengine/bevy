#[cfg(feature = "multi_threaded")]
use crate::batching::BatchingStrategy;
use crate::message::{Message, MessageCursor, MessageId, MessageInstance, Messages};
use core::{iter::Chain, slice::Iter};

/// An iterator that yields any unread messages from a [`MessageReader`](super::MessageReader) or [`MessageCursor`].
#[derive(Debug)]
pub struct MessageIterator<'a, M: Message> {
    iter: MessageIteratorWithId<'a, M>,
}

impl<'a, M: Message> Iterator for MessageIterator<'a, M> {
    type Item = &'a M;
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

impl<'a, M: Message> ExactSizeIterator for MessageIterator<'a, M> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

/// An iterator that yields any unread messages (and their IDs) from a [`MessageReader`](super::MessageReader) or [`MessageCursor`].
#[derive(Debug)]
pub struct MessageIteratorWithId<'a, M: Message> {
    reader: &'a mut MessageCursor<M>,
    chain: Chain<Iter<'a, MessageInstance<M>>, Iter<'a, MessageInstance<M>>>,
    unread: usize,
}

impl<'a, M: Message> MessageIteratorWithId<'a, M> {
    /// Creates a new iterator that yields any `messages` that have not yet been seen by `reader`.
    pub fn new(reader: &'a mut MessageCursor<M>, messages: &'a Messages<M>) -> Self {
        let a_index = reader
            .last_message_count
            .saturating_sub(messages.messages_a.start_message_count);
        let b_index = reader
            .last_message_count
            .saturating_sub(messages.messages_b.start_message_count);
        let a = messages.messages_a.get(a_index..).unwrap_or_default();
        let b = messages.messages_b.get(b_index..).unwrap_or_default();

        let unread_count = a.len() + b.len();
        // Ensure `len` is implemented correctly
        debug_assert_eq!(unread_count, reader.len(messages));
        reader.last_message_count = messages.message_count - unread_count;
        // Iterate the oldest first, then the newer messages
        let chain = a.iter().chain(b.iter());

        Self {
            reader,
            chain,
            unread: unread_count,
        }
    }

    /// Iterate over only the messages.
    pub fn without_id(self) -> MessageIterator<'a, M> {
        MessageIterator { iter: self }
    }
}

impl<'a, M: Message> Iterator for MessageIteratorWithId<'a, M> {
    type Item = (&'a M, MessageId<M>);
    fn next(&mut self) -> Option<Self::Item> {
        match self
            .chain
            .next()
            .map(|instance| (&instance.message, instance.message_id))
        {
            Some(item) => {
                #[cfg(feature = "detailed_trace")]
                tracing::trace!("MessageReader::iter() -> {}", item.1);
                self.reader.last_message_count += 1;
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
        self.reader.last_message_count += self.unread;
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
        self.reader.last_message_count += self.unread;
        Some((message, *message_id))
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        if let Some(MessageInstance {
            message_id,
            message,
        }) = self.chain.nth(n)
        {
            self.reader.last_message_count += n + 1;
            self.unread -= n + 1;
            Some((message, *message_id))
        } else {
            self.reader.last_message_count += self.unread;
            self.unread = 0;
            None
        }
    }
}

impl<'a, M: Message> ExactSizeIterator for MessageIteratorWithId<'a, M> {
    fn len(&self) -> usize {
        self.unread
    }
}

/// A parallel iterator over `Message`s.
#[cfg(feature = "multi_threaded")]
#[derive(Debug)]
pub struct MessageParIter<'a, M: Message> {
    reader: &'a mut MessageCursor<M>,
    slices: [&'a [MessageInstance<M>]; 2],
    batching_strategy: BatchingStrategy,
    #[cfg(not(target_arch = "wasm32"))]
    unread: usize,
}

#[cfg(feature = "multi_threaded")]
impl<'a, M: Message> MessageParIter<'a, M> {
    /// Creates a new parallel iterator over `messages` that have not yet been seen by `reader`.
    pub fn new(reader: &'a mut MessageCursor<M>, messages: &'a Messages<M>) -> Self {
        let a_index = reader
            .last_message_count
            .saturating_sub(messages.messages_a.start_message_count);
        let b_index = reader
            .last_message_count
            .saturating_sub(messages.messages_b.start_message_count);
        let a = messages.messages_a.get(a_index..).unwrap_or_default();
        let b = messages.messages_b.get(b_index..).unwrap_or_default();

        let unread_count = a.len() + b.len();
        // Ensure `len` is implemented correctly
        debug_assert_eq!(unread_count, reader.len(messages));
        reader.last_message_count = messages.message_count - unread_count;

        Self {
            reader,
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
    pub fn for_each<FN: Fn(&'a M) + Send + Sync + Clone>(self, func: FN) {
        self.for_each_with_id(move |e, _| func(e));
    }

    /// Runs the provided closure for each unread message in parallel, like [`for_each`](Self::for_each),
    /// but additionally provides the [`MessageId`] to the closure.
    ///
    /// Note that the order of iteration is not guaranteed, but [`MessageId`]s are ordered by send order.
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
    pub fn for_each_with_id<FN: Fn(&'a M, MessageId<M>) + Send + Sync + Clone>(mut self, func: FN) {
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
            let chunks = self.slices.map(|s| s.chunks_exact(batch_size));
            let remainders = chunks.each_ref().map(core::slice::ChunksExact::remainder);

            pool.scope(|scope| {
                for batch in chunks.into_iter().flatten().chain(remainders) {
                    let func = func.clone();
                    scope.spawn(async move {
                        for message_instance in batch {
                            func(&message_instance.message, message_instance.message_id);
                        }
                    });
                }
            });

            // Messages are guaranteed to be read at this point.
            self.reader.last_message_count += self.unread;
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
impl<'a, M: Message> IntoIterator for MessageParIter<'a, M> {
    type IntoIter = MessageIteratorWithId<'a, M>;
    type Item = <Self::IntoIter as Iterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        let MessageParIter {
            reader,
            slices: [a, b],
            ..
        } = self;
        let unread = a.len() + b.len();
        let chain = a.iter().chain(b);
        MessageIteratorWithId {
            reader,
            chain,
            unread,
        }
    }
}
