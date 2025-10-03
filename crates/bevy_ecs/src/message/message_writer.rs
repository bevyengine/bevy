use crate::{
    message::{Message, MessageId, Messages, WriteBatchIds},
    system::{ResMut, SystemParam},
};

/// Writes [`Message`]s of type `T`.
///
/// # Usage
///
/// `MessageWriter`s are usually declared as a [`SystemParam`].
/// ```
/// # use bevy_ecs::prelude::*;
///
/// #[derive(Message)]
/// pub struct MyMessage; // Custom message type.
/// fn my_system(mut writer: MessageWriter<MyMessage>) {
///     writer.write(MyMessage);
/// }
///
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
///
/// # Concurrency
///
/// `MessageWriter` param has [`ResMut<Messages<T>>`](Messages) inside. So two systems declaring `MessageWriter<T>` params
/// for the same message type won't be executed concurrently.
///
/// # Untyped messages
///
/// `MessageWriter` can only write messages of one specific type, which must be known at compile-time.
/// This is not a problem most of the time, but you may find a situation where you cannot know
/// ahead of time every kind of message you'll need to write. In this case, you can use the "type-erased message" pattern.
///
/// ```
/// # use bevy_ecs::{prelude::*, message::Messages};
/// # #[derive(Message)]
/// # pub struct MyMessage;
/// fn write_untyped(mut commands: Commands) {
///     // Write a message of a specific type without having to declare that
///     // type as a SystemParam.
///     //
///     // Effectively, we're just moving the type parameter from the /type/ to the /method/,
///     // which allows one to do all kinds of clever things with type erasure, such as sending
///     // custom messages to unknown 3rd party plugins (modding API).
///     //
///     // NOTE: the message won't actually be sent until commands get applied during
///     // apply_deferred.
///     commands.queue(|w: &mut World| {
///         w.write_message(MyMessage);
///     });
/// }
/// ```
/// Note that this is considered *non-idiomatic*, and should only be used when `MessageWriter` will not work.
///
/// [`Observer`]: crate::observer::Observer
#[derive(SystemParam)]
pub struct MessageWriter<'w, E: Message> {
    #[system_param(validation_message = "Message not initialized")]
    messages: ResMut<'w, Messages<E>>,
}

impl<'w, E: Message> MessageWriter<'w, E> {
    /// Writes an `message`, which can later be read by [`MessageReader`](super::MessageReader)s.
    /// This method returns the [ID](`MessageId`) of the written `message`.
    ///
    /// See [`Messages`] for details.
    #[doc(alias = "send")]
    #[track_caller]
    pub fn write(&mut self, message: E) -> MessageId<E> {
        self.messages.write(message)
    }

    /// Writes a list of `messages` all at once, which can later be read by [`MessageReader`](super::MessageReader)s.
    /// This is more efficient than writing each message individually.
    /// This method returns the [IDs](`MessageId`) of the written `messages`.
    ///
    /// See [`Messages`] for details.
    #[doc(alias = "send_batch")]
    #[track_caller]
    pub fn write_batch(&mut self, messages: impl IntoIterator<Item = E>) -> WriteBatchIds<E> {
        self.messages.write_batch(messages)
    }

    /// Writes the default value of the message. Useful when the message is an empty struct.
    /// This method returns the [ID](`MessageId`) of the written `message`.
    ///
    /// See [`Messages`] for details.
    #[doc(alias = "send_default")]
    #[track_caller]
    pub fn write_default(&mut self) -> MessageId<E>
    where
        E: Default,
    {
        self.messages.write_default()
    }
}
