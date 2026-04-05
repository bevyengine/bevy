#[cfg(feature = "multi_threaded")]
use crate::message::MessageMutParIter;
use crate::{
    message::{
        Message, MessageCursor, MessageId, MessageMutIterator, MessageMutIteratorWithId, Messages,
        WriteBatchIds,
    },
    system::{Local, ResMut, SystemParam},
};

/// Reads and writes [`Message`]s of type `T`, keeping track of which messages have already been read.
///
/// This can be used if a system needs to both read and write messages of the same type.
///
/// Since it has exclusive access to the underlying messages, it also permits messages to be modified as they are read.
/// This is ideal for chains of systems that all want to modify the same messages.
///
/// # Usage
///
/// [`MessageMutator`]s are usually declared as a [`SystemParam`].
/// ```
/// # use bevy_ecs::prelude::*;
///
/// #[derive(Message, Debug)]
/// pub struct MyMessage(pub u32); // Custom message type.
/// fn my_system(mut mutator: MessageMutator<MyMessage>) {
///     // This message will be read immediately by this system,
///     // and will then be visible to other systems.
///     mutator.write(MyMessage(0));
///     for message in mutator.read() {
///         message.0 += 1;
///         println!("received message: {:?}", message);
///     }
///     // This message will be read on the next run of this system,
///     // but will be visible immediately to other systems.
///     mutator.write(MyMessage(0));
/// }
/// ```
///
/// # Concurrency
///
/// Multiple systems with `MessageMutator<T>` of the same message type can not run concurrently.
/// They also can not be executed in parallel with [`MessageReader`] or [`MessageWriter`].
///
/// # Clearing, Reading, and Peeking
///
/// Messages are stored in a double buffered queue that switches each frame. This switch also clears the previous
/// frame's messages. Messages should be read each frame otherwise they may be lost. For manual control over this
/// behavior, see [`Messages`].
///
/// Most of the time systems will want to use [`MessageMutator::read()`]. This function creates an iterator over
/// all messages that haven't been read yet by this system, marking the message as read in the process.
///
/// [`MessageReader`]: super::MessageReader
/// [`MessageWriter`]: super::MessageWriter
#[derive(SystemParam, Debug)]
pub struct MessageMutator<'w, 's, M: Message> {
    pub(super) reader: Local<'s, MessageCursor<M>>,
    #[system_param(validation_message = "Message not initialized")]
    messages: ResMut<'w, Messages<M>>,
}

impl<'w, 's, M: Message> MessageMutator<'w, 's, M> {
    /// Iterates over the messages this [`MessageMutator`] has not seen yet. This updates the
    /// [`MessageMutator`]'s message counter, which means subsequent message reads will not include messages
    /// that happened before now.
    pub fn read(&mut self) -> MessageMutIterator<'_, M> {
        self.reader.read_mut(&mut self.messages)
    }

    /// Like [`read`](Self::read), except also returning the [`MessageId`] of the messages.
    pub fn read_with_id(&mut self) -> MessageMutIteratorWithId<'_, M> {
        self.reader.read_mut_with_id(&mut self.messages)
    }

    /// Returns a parallel iterator over the messages this [`MessageMutator`] has not seen yet.
    /// See also [`for_each`](super::MessageParIter::for_each).
    ///
    /// # Example
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use std::sync::atomic::{AtomicUsize, Ordering};
    ///
    /// #[derive(Message)]
    /// struct MyMessage {
    ///     value: usize,
    /// }
    ///
    /// #[derive(Resource, Default)]
    /// struct Counter(AtomicUsize);
    ///
    /// // setup
    /// let mut world = World::new();
    /// world.init_resource::<Messages<MyMessage>>();
    /// world.insert_resource(Counter::default());
    ///
    /// let mut schedule = Schedule::default();
    /// schedule.add_systems(|mut messages: MessageMutator<MyMessage>, counter: Res<Counter>| {
    ///     messages.par_read().for_each(|MyMessage { value }| {
    ///         counter.0.fetch_add(*value, Ordering::Relaxed);
    ///     });
    /// });
    /// for value in 0..100 {
    ///     world.write_message(MyMessage { value });
    /// }
    /// schedule.run(&mut world);
    /// let Counter(counter) = world.remove_resource::<Counter>().unwrap();
    /// // all messages were processed
    /// assert_eq!(counter.into_inner(), 4950);
    /// ```
    #[cfg(feature = "multi_threaded")]
    pub fn par_read(&mut self) -> MessageMutParIter<'_, M> {
        self.reader.par_read_mut(&mut self.messages)
    }

    /// Determines the number of messages available to be read from this [`MessageMutator`] without consuming any.
    pub fn len(&self) -> usize {
        self.reader.len(&self.messages)
    }

    /// Returns `true` if there are no messages available to read.
    ///
    /// # Example
    ///
    /// The following example shows a useful pattern where some behavior is triggered if new messages are available.
    /// [`MessageMutator::clear()`] is used so the same messages don't re-trigger the behavior the next time the system runs.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// #[derive(Message)]
    /// struct Collision;
    ///
    /// fn play_collision_sound(mut messages: MessageMutator<Collision>) {
    ///     if !messages.is_empty() {
    ///         messages.clear();
    ///         // Play a sound
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(play_collision_sound);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.reader.is_empty(&self.messages)
    }

    /// Consumes all available messages.
    ///
    /// This means these messages will not appear in calls to [`MessageMutator::read()`] or
    /// [`MessageMutator::read_with_id()`] and [`MessageMutator::is_empty()`] will return `true`.
    ///
    /// For usage, see [`MessageMutator::is_empty()`].
    pub fn clear(&mut self) {
        self.reader.clear(&self.messages);
    }

    /// Writes an `message`, which can later be read by [`MessageReader`](super::MessageReader)s.
    /// This method returns the [ID](`MessageId`) of the written `message`.
    ///
    /// See [`Messages`] for details.
    #[track_caller]
    pub fn write(&mut self, message: M) -> MessageId<M> {
        self.messages.write(message)
    }

    /// Writes a list of `messages` all at once, which can later be read by [`MessageReader`](super::MessageReader)s.
    /// This is more efficient than writing each message individually.
    /// This method returns the [IDs](`MessageId`) of the written `messages`.
    ///
    /// See [`Messages`] for details.
    #[track_caller]
    pub fn write_batch(&mut self, messages: impl IntoIterator<Item = M>) -> WriteBatchIds<M> {
        self.messages.write_batch(messages)
    }

    /// Writes the default value of the message. Useful when the message is an empty struct.
    /// This method returns the [ID](`MessageId`) of the written `message`.
    ///
    /// See [`Messages`] for details.
    #[track_caller]
    pub fn write_default(&mut self) -> MessageId<M>
    where
        M: Default,
    {
        self.messages.write_default()
    }
}
