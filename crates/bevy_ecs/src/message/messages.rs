use crate::{
    change_detection::MaybeLocation,
    message::{Message, MessageCursor, MessageId, MessageInstance},
    resource::Resource,
};
use alloc::vec::Vec;
use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};
#[cfg(feature = "bevy_reflect")]
use {
    crate::reflect::ReflectResource,
    bevy_reflect::{std_traits::ReflectDefault, Reflect},
};

/// A message collection that represents the messages that occurred within the last two
/// [`Messages::update`] calls.
/// Messages can be written to using a [`MessageWriter`]
/// and are typically cheaply read using a [`MessageReader`].
///
/// Each message can be consumed by multiple systems, in parallel,
/// with consumption tracked by the [`MessageReader`] on a per-system basis.
///
/// If no [ordering](https://github.com/bevyengine/bevy/blob/main/examples/ecs/ecs_guide.rs)
/// is applied between writing and reading systems, there is a risk of a race condition.
/// This means that whether the messages arrive before or after the next [`Messages::update`] is unpredictable.
///
/// This collection is meant to be paired with a system that calls
/// [`Messages::update`] exactly once per update/frame.
///
/// [`message_update_system`] is a system that does this, typically initialized automatically using
/// [`add_message`](https://docs.rs/bevy/*/bevy/app/struct.App.html#method.add_message).
/// [`MessageReader`]s are expected to read messages from this collection at least once per loop/frame.
/// Messages will persist across a single frame boundary and so ordering of message producers and
/// consumers is not critical (although poorly-planned ordering may cause accumulating lag).
/// If messages are not handled by the end of the frame after they are updated, they will be
/// dropped silently.
///
/// # Example
///
/// ```
/// use bevy_ecs::message::{Message, Messages};
///
/// #[derive(Message)]
/// struct MyMessage {
///     value: usize
/// }
///
/// // setup
/// let mut messages = Messages::<MyMessage>::default();
/// let mut cursor = messages.get_cursor();
///
/// // run this once per update/frame
/// messages.update();
///
/// // somewhere else: write a message
/// messages.write(MyMessage { value: 1 });
///
/// // somewhere else: read the messages
/// for message in cursor.read(&messages) {
///     assert_eq!(message.value, 1)
/// }
///
/// // messages are only processed once per reader
/// assert_eq!(cursor.read(&messages).count(), 0);
/// ```
///
/// # Details
///
/// [`Messages`] is implemented using a variation of a double buffer strategy.
/// Each call to [`update`](Messages::update) swaps buffers and clears out the oldest one.
/// - [`MessageReader`]s will read messages from both buffers.
/// - [`MessageReader`]s that read at least once per update will never drop messages.
/// - [`MessageReader`]s that read once within two updates might still receive some messages
/// - [`MessageReader`]s that read after two updates are guaranteed to drop all messages that occurred
///   before those updates.
///
/// The buffers in [`Messages`] will grow indefinitely if [`update`](Messages::update) is never called.
///
/// An alternative call pattern would be to call [`update`](Messages::update)
/// manually across frames to control when messages are cleared.
/// This complicates consumption and risks ever-expanding memory usage if not cleaned up,
/// but can be done by adding your message as a resource instead of using
/// [`add_message`](https://docs.rs/bevy/*/bevy/app/struct.App.html#method.add_message).
///
/// [Example usage.](https://github.com/bevyengine/bevy/blob/latest/examples/ecs/message.rs)
/// [Example usage standalone.](https://github.com/bevyengine/bevy/blob/latest/crates/bevy_ecs/examples/messages.rs)
///
/// [`MessageReader`]: super::MessageReader
/// [`MessageWriter`]: super::MessageWriter
/// [`message_update_system`]: super::message_update_system
#[derive(Debug, Resource)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Resource, Default))]
pub struct Messages<E: Message> {
    /// Holds the oldest still active messages.
    /// Note that `a.start_message_count + a.len()` should always be equal to `messages_b.start_message_count`.
    pub(crate) messages_a: MessageSequence<E>,
    /// Holds the newer messages.
    pub(crate) messages_b: MessageSequence<E>,
    pub(crate) message_count: usize,
}

// Derived Default impl would incorrectly require E: Default
impl<E: Message> Default for Messages<E> {
    fn default() -> Self {
        Self {
            messages_a: Default::default(),
            messages_b: Default::default(),
            message_count: Default::default(),
        }
    }
}

impl<M: Message> Messages<M> {
    /// Returns the index of the oldest message stored in the message buffer.
    pub fn oldest_message_count(&self) -> usize {
        self.messages_a.start_message_count
    }

    /// Writes an `message` to the current message buffer.
    /// [`MessageReader`](super::MessageReader)s can then read the message.
    /// This method returns the [ID](`MessageId`) of the written `message`.
    #[track_caller]
    pub fn write(&mut self, message: M) -> MessageId<M> {
        self.write_with_caller(message, MaybeLocation::caller())
    }

    pub(crate) fn write_with_caller(&mut self, message: M, caller: MaybeLocation) -> MessageId<M> {
        let message_id = MessageId {
            id: self.message_count,
            caller,
            _marker: PhantomData,
        };
        #[cfg(feature = "detailed_trace")]
        tracing::trace!("Messages::write() -> id: {}", message_id);

        let message_instance = MessageInstance {
            message_id,
            message,
        };

        self.messages_b.push(message_instance);
        self.message_count += 1;

        message_id
    }

    /// Writes a list of `messages` all at once, which can later be read by [`MessageReader`](super::MessageReader)s.
    /// This is more efficient than writing each message individually.
    /// This method returns the [IDs](`MessageId`) of the written `messages`.
    #[track_caller]
    pub fn write_batch(&mut self, messages: impl IntoIterator<Item = M>) -> WriteBatchIds<M> {
        let last_count = self.message_count;

        self.extend(messages);

        WriteBatchIds {
            last_count,
            message_count: self.message_count,
            _marker: PhantomData,
        }
    }

    /// Writes the default value of the message. Useful when the message is an empty struct.
    /// This method returns the [ID](`MessageId`) of the written `message`.
    #[track_caller]
    pub fn write_default(&mut self) -> MessageId<M>
    where
        M: Default,
    {
        self.write(Default::default())
    }

    /// "Sends" an `message` by writing it to the current message buffer.
    /// [`MessageReader`](super::MessageReader)s can then read the message.
    /// This method returns the [ID](`MessageId`) of the sent `message`.
    #[deprecated(since = "0.17.0", note = "Use `Messages<E>::write` instead.")]
    #[track_caller]
    pub fn send(&mut self, message: M) -> MessageId<M> {
        self.write(message)
    }

    /// Sends a list of `messages` all at once, which can later be read by [`MessageReader`](super::MessageReader)s.
    /// This is more efficient than sending each message individually.
    /// This method returns the [IDs](`MessageId`) of the sent `messages`.
    #[deprecated(since = "0.17.0", note = "Use `Messages<E>::write_batch` instead.")]
    #[track_caller]
    pub fn send_batch(&mut self, messages: impl IntoIterator<Item = M>) -> WriteBatchIds<M> {
        self.write_batch(messages)
    }

    /// Sends the default value of the message. Useful when the message is an empty struct.
    /// This method returns the [ID](`MessageId`) of the sent `message`.
    #[deprecated(since = "0.17.0", note = "Use `Messages<E>::write_default` instead.")]
    #[track_caller]
    pub fn send_default(&mut self) -> MessageId<M>
    where
        M: Default,
    {
        self.write_default()
    }

    /// Gets a new [`MessageCursor`]. This will include all messages already in the message buffers.
    pub fn get_cursor(&self) -> MessageCursor<M> {
        MessageCursor::default()
    }

    /// Gets a new [`MessageCursor`]. This will ignore all messages already in the message buffers.
    /// It will read all future messages.
    pub fn get_cursor_current(&self) -> MessageCursor<M> {
        MessageCursor {
            last_message_count: self.message_count,
            ..Default::default()
        }
    }

    /// Swaps the message buffers and clears the oldest message buffer. In general, this should be
    /// called once per frame/update.
    ///
    /// If you need access to the messages that were removed, consider using [`Messages::update_drain`].
    pub fn update(&mut self) {
        core::mem::swap(&mut self.messages_a, &mut self.messages_b);
        self.messages_b.clear();
        self.messages_b.start_message_count = self.message_count;
        debug_assert_eq!(
            self.messages_a.start_message_count + self.messages_a.len(),
            self.messages_b.start_message_count
        );
    }

    /// Swaps the message buffers and drains the oldest message buffer, returning an iterator
    /// of all messages that were removed. In general, this should be called once per frame/update.
    ///
    /// If you do not need to take ownership of the removed messages, use [`Messages::update`] instead.
    #[must_use = "If you do not need the returned messages, call .update() instead."]
    pub fn update_drain(&mut self) -> impl Iterator<Item = M> + '_ {
        core::mem::swap(&mut self.messages_a, &mut self.messages_b);
        let iter = self.messages_b.messages.drain(..);
        self.messages_b.start_message_count = self.message_count;
        debug_assert_eq!(
            self.messages_a.start_message_count + self.messages_a.len(),
            self.messages_b.start_message_count
        );

        iter.map(|e| e.message)
    }

    #[inline]
    fn reset_start_message_count(&mut self) {
        self.messages_a.start_message_count = self.message_count;
        self.messages_b.start_message_count = self.message_count;
    }

    /// Removes all messages.
    #[inline]
    pub fn clear(&mut self) {
        self.reset_start_message_count();
        self.messages_a.clear();
        self.messages_b.clear();
    }

    /// Returns the number of messages currently stored in the message buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.messages_a.len() + self.messages_b.len()
    }

    /// Returns true if there are no messages currently stored in the message buffer.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Creates a draining iterator that removes all messages.
    pub fn drain(&mut self) -> impl Iterator<Item = M> + '_ {
        self.reset_start_message_count();

        // Drain the oldest messages first, then the newest
        self.messages_a
            .drain(..)
            .chain(self.messages_b.drain(..))
            .map(|i| i.message)
    }

    /// Iterates over messages that happened since the last "update" call.
    /// WARNING: You probably don't want to use this call. In most cases you should use an
    /// [`MessageReader`]. You should only use this if you know you only need to consume messages
    /// between the last `update()` call and your call to `iter_current_update_messages`.
    /// If messages happen outside that window, they will not be handled. For example, any messages that
    /// happen after this call and before the next `update()` call will be dropped.
    ///
    /// [`MessageReader`]: super::MessageReader
    pub fn iter_current_update_messages(&self) -> impl ExactSizeIterator<Item = &M> {
        self.messages_b.iter().map(|i| &i.message)
    }

    /// Get a specific message by id if it still exists in the messages buffer.
    pub fn get_message(&self, id: usize) -> Option<(&M, MessageId<M>)> {
        if id < self.oldest_message_count() {
            return None;
        }

        let sequence = self.sequence(id);
        let index = id.saturating_sub(sequence.start_message_count);

        sequence
            .get(index)
            .map(|instance| (&instance.message, instance.message_id))
    }

    /// Which message buffer is this message id a part of.
    fn sequence(&self, id: usize) -> &MessageSequence<M> {
        if id < self.messages_b.start_message_count {
            &self.messages_a
        } else {
            &self.messages_b
        }
    }
}

impl<E: Message> Extend<E> for Messages<E> {
    #[track_caller]
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = E>,
    {
        let old_count = self.message_count;
        let mut message_count = self.message_count;
        let messages = iter.into_iter().map(|message| {
            let message_id = MessageId {
                id: message_count,
                caller: MaybeLocation::caller(),
                _marker: PhantomData,
            };
            message_count += 1;
            MessageInstance {
                message_id,
                message,
            }
        });

        self.messages_b.extend(messages);

        if old_count != message_count {
            #[cfg(feature = "detailed_trace")]
            tracing::trace!(
                "Messages::extend() -> ids: ({}..{})",
                self.message_count,
                message_count
            );
        }

        self.message_count = message_count;
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Default))]
pub(crate) struct MessageSequence<E: Message> {
    pub(crate) messages: Vec<MessageInstance<E>>,
    pub(crate) start_message_count: usize,
}

// Derived Default impl would incorrectly require E: Default
impl<E: Message> Default for MessageSequence<E> {
    fn default() -> Self {
        Self {
            messages: Default::default(),
            start_message_count: Default::default(),
        }
    }
}

impl<E: Message> Deref for MessageSequence<E> {
    type Target = Vec<MessageInstance<E>>;

    fn deref(&self) -> &Self::Target {
        &self.messages
    }
}

impl<E: Message> DerefMut for MessageSequence<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.messages
    }
}

/// [`Iterator`] over written [`MessageIds`](`MessageId`) from a batch.
pub struct WriteBatchIds<E> {
    last_count: usize,
    message_count: usize,
    _marker: PhantomData<E>,
}

/// [`Iterator`] over sent [`MessageIds`](`MessageId`) from a batch.
#[deprecated(since = "0.17.0", note = "Use `WriteBatchIds` instead.")]
pub type SendBatchIds<E> = WriteBatchIds<E>;

impl<E: Message> Iterator for WriteBatchIds<E> {
    type Item = MessageId<E>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.last_count >= self.message_count {
            return None;
        }

        let result = Some(MessageId {
            id: self.last_count,
            caller: MaybeLocation::caller(),
            _marker: PhantomData,
        });

        self.last_count += 1;

        result
    }
}

impl<E: Message> ExactSizeIterator for WriteBatchIds<E> {
    fn len(&self) -> usize {
        self.message_count.saturating_sub(self.last_count)
    }
}

#[cfg(test)]
mod tests {
    use crate::message::{Message, Messages};

    #[test]
    fn iter_current_update_messages_iterates_over_current_messages() {
        #[derive(Message, Clone)]
        struct TestMessage;

        let mut test_messages = Messages::<TestMessage>::default();

        // Starting empty
        assert_eq!(test_messages.len(), 0);
        assert_eq!(test_messages.iter_current_update_messages().count(), 0);
        test_messages.update();

        // Writing one message
        test_messages.write(TestMessage);

        assert_eq!(test_messages.len(), 1);
        assert_eq!(test_messages.iter_current_update_messages().count(), 1);
        test_messages.update();

        // Writing two messages on the next frame
        test_messages.write(TestMessage);
        test_messages.write(TestMessage);

        assert_eq!(test_messages.len(), 3); // Messages are double-buffered, so we see 1 + 2 = 3
        assert_eq!(test_messages.iter_current_update_messages().count(), 2);
        test_messages.update();

        // Writing zero messages
        assert_eq!(test_messages.len(), 2); // Messages are double-buffered, so we see 2 + 0 = 2
        assert_eq!(test_messages.iter_current_update_messages().count(), 0);
    }
}
