use crate::message::{
    Message, MessageIterator, MessageIteratorWithId, MessageMutIterator, MessageMutIteratorWithId,
    Messages,
};
#[cfg(feature = "multi_threaded")]
use crate::message::{MessageMutParIter, MessageParIter};
use core::marker::PhantomData;

/// Stores the state for a [`MessageReader`] or [`MessageMutator`].
///
/// Access to the [`Messages<M>`] resource is required to read any incoming messages.
///
/// In almost all cases, you should just use a [`MessageReader`] or [`MessageMutator`],
/// which will automatically manage the state for you.
///
/// However, this type can be useful if you need to manually track messages,
/// such as when you're attempting to send and receive messages of the same type in the same system.
///
/// # Example
///
/// ```
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::message::{Message, MessageCursor};
///
/// #[derive(Message, Clone, Debug)]
/// struct MyMessage;
///
/// /// A system that both sends and receives messages using a [`Local`] [`MessageCursor`].
/// fn send_and_receive_messages(
///     // The `Local` `SystemParam` stores state inside the system itself, rather than in the world.
///     // `MessageCursor<T>` is the internal state of `MessageMutator<T>`, which tracks which messages have been seen.
///     mut local_message_reader: Local<MessageCursor<MyMessage>>,
///     // We can access the `Messages` resource mutably, allowing us to both read and write its contents.
///     mut messages: ResMut<Messages<MyMessage>>,
/// ) {
///     // We must collect the messages to resend, because we can't mutate messages while we're iterating over the messages.
///     let mut messages_to_resend = Vec::new();
///
///     for message in local_message_reader.read(&mut messages) {
///          messages_to_resend.push(message.clone());
///     }
///
///     for message in messages_to_resend {
///         messages.write(MyMessage);
///     }
/// }
///
/// # bevy_ecs::system::assert_is_system(send_and_receive_messages);
/// ```
///
/// [`MessageReader`]: super::MessageReader
/// [`MessageMutator`]: super::MessageMutator
#[derive(Debug)]
pub struct MessageCursor<M: Message> {
    pub(super) last_message_count: usize,
    pub(super) _marker: PhantomData<M>,
}

impl<M: Message> Default for MessageCursor<M> {
    fn default() -> Self {
        MessageCursor {
            last_message_count: 0,
            _marker: Default::default(),
        }
    }
}

impl<M: Message> Clone for MessageCursor<M> {
    fn clone(&self) -> Self {
        MessageCursor {
            last_message_count: self.last_message_count,
            _marker: PhantomData,
        }
    }
}

impl<M: Message> MessageCursor<M> {
    /// See [`MessageReader::read`](super::MessageReader::read)
    pub fn read<'a>(&'a mut self, messages: &'a Messages<M>) -> MessageIterator<'a, M> {
        self.read_with_id(messages).without_id()
    }

    /// See [`MessageMutator::read`](super::MessageMutator::read)
    pub fn read_mut<'a>(&'a mut self, messages: &'a mut Messages<M>) -> MessageMutIterator<'a, M> {
        self.read_mut_with_id(messages).without_id()
    }

    /// See [`MessageReader::read_with_id`](super::MessageReader::read_with_id)
    pub fn read_with_id<'a>(
        &'a mut self,
        messages: &'a Messages<M>,
    ) -> MessageIteratorWithId<'a, M> {
        MessageIteratorWithId::new(self, messages)
    }

    /// See [`MessageMutator::read_with_id`](super::MessageMutator::read_with_id)
    pub fn read_mut_with_id<'a>(
        &'a mut self,
        messages: &'a mut Messages<M>,
    ) -> MessageMutIteratorWithId<'a, M> {
        MessageMutIteratorWithId::new(self, messages)
    }

    /// See [`MessageReader::par_read`](super::MessageReader::par_read)
    #[cfg(feature = "multi_threaded")]
    pub fn par_read<'a>(&'a mut self, messages: &'a Messages<M>) -> MessageParIter<'a, M> {
        MessageParIter::new(self, messages)
    }

    /// See [`MessageMutator::par_read`](super::MessageMutator::par_read)
    #[cfg(feature = "multi_threaded")]
    pub fn par_read_mut<'a>(
        &'a mut self,
        messages: &'a mut Messages<M>,
    ) -> MessageMutParIter<'a, M> {
        MessageMutParIter::new(self, messages)
    }

    /// See [`MessageReader::len`](super::MessageReader::len)
    pub fn len(&self, messages: &Messages<M>) -> usize {
        // The number of messages in this reader is the difference between the most recent message
        // and the last message seen by it. This will be at most the number of messages contained
        // with the messages (any others have already been dropped)
        // TODO: Warn when there are dropped messages, or return e.g. a `Result<usize, (usize, usize)>`
        messages
            .message_count
            .saturating_sub(self.last_message_count)
            .min(messages.len())
    }

    /// Amount of messages we missed.
    pub fn missed_messages(&self, messages: &Messages<M>) -> usize {
        messages
            .oldest_message_count()
            .saturating_sub(self.last_message_count)
    }

    /// See [`MessageReader::is_empty()`](super::MessageReader::is_empty)
    pub fn is_empty(&self, messages: &Messages<M>) -> bool {
        self.len(messages) == 0
    }

    /// See [`MessageReader::clear()`](super::MessageReader::clear)
    pub fn clear(&mut self, messages: &Messages<M>) {
        self.last_message_count = messages.message_count;
    }
}
