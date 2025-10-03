#[cfg(feature = "multi_threaded")]
use crate::message::MessageMutParIter;
use crate::{
    message::{Message, MessageCursor, MessageMutIterator, MessageMutIteratorWithId, Messages},
    system::{Local, ResMut, SystemParam},
};

/// Mutably reads messages of type `T` keeping track of which messages have already been read
/// by each system allowing multiple systems to read the same messages. Ideal for chains of systems
/// that all want to modify the same messages.
///
/// # Usage
///
/// [`MessageMutator`]s are usually declared as a [`SystemParam`].
/// ```
/// # use bevy_ecs::prelude::*;
///
/// #[derive(Message, Debug)]
/// pub struct MyMessage(pub u32); // Custom message type.
/// fn my_system(mut reader: MessageMutator<MyMessage>) {
///     for message in reader.read() {
///         message.0 += 1;
///         println!("received message: {:?}", message);
///     }
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
pub struct MessageMutator<'w, 's, E: Message> {
    pub(super) reader: Local<'s, MessageCursor<E>>,
    #[system_param(validation_message = "Message not initialized")]
    messages: ResMut<'w, Messages<E>>,
}

impl<'w, 's, E: Message> MessageMutator<'w, 's, E> {
    /// Iterates over the messages this [`MessageMutator`] has not seen yet. This updates the
    /// [`MessageMutator`]'s message counter, which means subsequent message reads will not include messages
    /// that happened before now.
    pub fn read(&mut self) -> MessageMutIterator<'_, E> {
        self.reader.read_mut(&mut self.messages)
    }

    /// Like [`read`](Self::read), except also returning the [`MessageId`](super::MessageId) of the messages.
    pub fn read_with_id(&mut self) -> MessageMutIteratorWithId<'_, E> {
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
    pub fn par_read(&mut self) -> MessageMutParIter<'_, E> {
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
}
