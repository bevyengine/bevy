#[cfg(feature = "multi_threaded")]
use crate::message::MessageParIter;
use crate::{
    message::{Message, MessageCursor, MessageIterator, MessageIteratorWithId, Messages},
    system::{Local, Res, SystemParam, SystemParamValidationError},
};

/// Reads [`Message`]s of type `T` in order and tracks which messages have already been read.
///
/// Use [`PopulatedMessageReader<T>`] to skip the system if there are no messages.
///
/// # Concurrency
///
/// Unlike [`MessageWriter<T>`], systems with `MessageReader<T>` param can be executed concurrently
/// (but not concurrently with `MessageWriter<T>` or `MessageMutator<T>` systems for the same message type).
///
/// [`MessageWriter<T>`]: super::MessageWriter
#[derive(SystemParam, Debug)]
pub struct MessageReader<'w, 's, M: Message> {
    pub(super) reader: Local<'s, MessageCursor<M>>,
    #[system_param(validation_message = "Message not initialized")]
    messages: Res<'w, Messages<M>>,
}

impl<'w, 's, M: Message> MessageReader<'w, 's, M> {
    /// Iterates over the messages this [`MessageReader`] has not seen yet. This updates the
    /// [`MessageReader`]'s message counter, which means subsequent message reads will not include messages
    /// that happened before now.
    pub fn read(&mut self) -> MessageIterator<'_, M> {
        self.reader.read(&self.messages)
    }

    /// Like [`read`](Self::read), except also returning the [`MessageId`](super::MessageId) of the messages.
    pub fn read_with_id(&mut self) -> MessageIteratorWithId<'_, M> {
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
    pub fn par_read(&mut self) -> MessageParIter<'_, M> {
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

/// Reads [`Message`]s of type `T` in order and tracks which messages have already been read.
/// Skips the system if there no messages.
///
/// Use [`MessageReader<T>`] to run the system even if there are no messages.
///
/// Use the [`on_message`](crate::prelude::on_message) run condition to skip the system based on messages that it doesn't read.
#[derive(Debug)]
pub struct PopulatedMessageReader<'w, 's, M: Message>(MessageReader<'w, 's, M>);

impl<'w, 's, M: Message> core::ops::Deref for PopulatedMessageReader<'w, 's, M> {
    type Target = MessageReader<'w, 's, M>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'w, 's, M: Message> core::ops::DerefMut for PopulatedMessageReader<'w, 's, M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// SAFETY: relies on MessageReader to uphold soundness requirements
unsafe impl<'w, 's, M: Message> SystemParam for PopulatedMessageReader<'w, 's, M> {
    type State = <MessageReader<'w, 's, M> as SystemParam>::State;
    type Item<'world, 'state> = PopulatedMessageReader<'world, 'state, M>;

    fn init_state(world: &mut crate::prelude::World) -> Self::State {
        MessageReader::<M>::init_state(world)
    }

    fn init_access(
        state: &Self::State,
        system_meta: &mut crate::system::SystemMeta,
        component_access_set: &mut crate::query::FilteredAccessSet,
        world: &mut crate::prelude::World,
    ) {
        MessageReader::<M>::init_access(state, system_meta, component_access_set, world);
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &crate::system::SystemMeta,
        world: crate::world::unsafe_world_cell::UnsafeWorldCell<'world>,
        change_tick: crate::change_detection::Tick,
    ) -> Self::Item<'world, 'state> {
        // SAFETY: requirements are upheld by MessageReader's implementation
        unsafe {
            PopulatedMessageReader(MessageReader::get_param(
                state,
                system_meta,
                world,
                change_tick,
            ))
        }
    }

    unsafe fn validate_param(
        state: &mut Self::State,
        system_meta: &crate::system::SystemMeta,
        world: crate::world::unsafe_world_cell::UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // SAFETY: requirements are upheld by MessageReader's implementation
        unsafe { MessageReader::<M>::validate_param(state, system_meta, world) }?;

        // SAFETY: requirements are upheld by MessageReader's implementation
        let reader =
            unsafe { MessageReader::get_param(state, system_meta, world, world.change_tick()) };
        if reader.is_empty() {
            Err(SystemParamValidationError::skipped::<Self>(
                "message queue is empty",
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::{AtomicBool, Ordering};

    use super::*;
    use crate::message::MessageRegistry;
    use crate::prelude::*;
    use bevy_platform::sync::Arc;

    #[test]
    fn test_populated_message_reader() {
        let system_ran = Arc::new(AtomicBool::new(false));

        let mut world = World::new();
        MessageRegistry::register_message::<TheMessage>(&mut world);

        let mut schedule = Schedule::default();
        schedule.add_systems({
            let system_ran = system_ran.clone();
            move |mut _reader: PopulatedMessageReader<TheMessage>| {
                system_ran.store(true, Ordering::SeqCst);
            }
        });

        schedule.run(&mut world);
        assert!(
            !system_ran.load(Ordering::SeqCst),
            "system with PopulatedMessageReader should have been skipped"
        );

        world.write_message(TheMessage);
        schedule.run(&mut world);
        assert!(
            system_ran.load(Ordering::SeqCst),
            "system with PopulatedMessageReader should NOT have been skipped"
        );

        #[derive(Message)]
        struct TheMessage;
    }
}
