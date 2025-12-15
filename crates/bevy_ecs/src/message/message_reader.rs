#[cfg(feature = "multi_threaded")]
use crate::message::MessageParIter;
use crate::{
    message::{Message, MessageCursor, MessageIterator, MessageIteratorWithId, Messages},
    system::{Local, Res, SystemParam},
};

/// Reads [`Message`]s of type `T` in order and tracks which messages have already been read.
///
/// # Concurrency
///
/// Unlike [`MessageWriter<T>`], systems with `MessageReader<T>` param can be executed concurrently
/// (but not concurrently with `MessageWriter<T>` or `MessageMutator<T>` systems for the same message type).
///
/// [`MessageWriter<T>`]: super::MessageWriter
#[derive(SystemParam, Debug)]
pub struct MessageReader<'w, 's, E: Message> {
    pub(super) reader: Local<'s, MessageCursor<E>>,
    #[system_param(validation_message = "Message not initialized")]
    messages: Res<'w, Messages<E>>,
}

impl<'w, 's, E: Message> MessageReader<'w, 's, E> {
    /// Iterates over the messages this [`MessageReader`] has not seen yet. This updates the
    /// [`MessageReader`]'s message counter, which means subsequent message reads will not include messages
    /// that happened before now.
    pub fn read(&mut self) -> MessageIterator<'_, E> {
        self.reader.read(&self.messages)
    }

    /// Like [`read`](Self::read), except also returning the [`MessageId`](super::MessageId) of the messages.
    pub fn read_with_id(&mut self) -> MessageIteratorWithId<'_, E> {
        self.reader.read_with_id(&self.messages)
    }

    /// Returns a parallel iterator over the messages this [`MessageReader`] has not seen yet.
    /// See also [`for_each`](MessageParIter::for_each).
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
    /// schedule.add_systems(|mut messages: MessageReader<MyMessage>, counter: Res<Counter>| {
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
    pub fn par_read(&mut self) -> MessageParIter<'_, E> {
        self.reader.par_read(&self.messages)
    }

    /// Determines the number of messages available to be read from this [`MessageReader`] without consuming any.
    pub fn len(&self) -> usize {
        self.reader.len(&self.messages)
    }

    /// Returns `true` if there are no messages available to read.
    ///
    /// # Example
    ///
    /// The following example shows a useful pattern where some behavior is triggered if new messages are available.
    /// [`MessageReader::clear()`] is used so the same messages don't re-trigger the behavior the next time the system runs.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// #[derive(Message)]
    /// struct Collision;
    ///
    /// fn play_collision_sound(mut messages: MessageReader<Collision>) {
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
    /// This means these messages will not appear in calls to [`MessageReader::read()`] or
    /// [`MessageReader::read_with_id()`] and [`MessageReader::is_empty()`] will return `true`.
    ///
    /// For usage, see [`MessageReader::is_empty()`].
    pub fn clear(&mut self) {
        self.reader.clear(&self.messages);
    }
}
