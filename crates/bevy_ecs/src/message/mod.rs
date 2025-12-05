//! [`Message`] functionality.

mod iterators;
mod message_cursor;
mod message_mutator;
mod message_reader;
mod message_registry;
mod message_writer;
mod messages;
mod mut_iterators;
mod update;

pub use iterators::*;
pub use message_cursor::*;
pub use message_mutator::*;
pub use message_reader::*;
pub use message_registry::*;
pub use message_writer::*;
pub use messages::*;
pub use mut_iterators::*;
pub use update::*;

pub use bevy_ecs_macros::Message;

use crate::change_detection::MaybeLocation;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use core::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

/// A buffered message for pull-based event handling.
///
/// Messages can be written with [`MessageWriter`] and read using the [`MessageReader`] system parameter.
/// Messages are stored in the [`Messages<M>`] resource, and require periodically polling the world for new messages,
/// typically in a system that runs as part of a schedule.
///
/// While the polling imposes a small overhead, messages are useful for efficiently batch processing
/// a large number of messages at once. For cases like these, messages can be more efficient than [`Event`]s (which are handled via [`Observer`]s).
///
/// Unlike [`Event`]s triggered for observers, messages are evaluated at fixed points in the schedule
/// rather than immediately when they are sent. This allows for more predictable scheduling, and deferring
/// message processing to a later point in time.
///
/// Messages must be thread-safe.
///
/// # Usage
///
/// The [`Message`] trait can be derived:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// #[derive(Message)]
/// struct Greeting(String);
/// ```
///
/// The message can then be written to the message buffer using a [`MessageWriter`]:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(Message)]
/// # struct Greeting(String);
/// #
/// fn write_hello(mut writer: MessageWriter<Greeting>) {
///     writer.write(Greeting("Hello!".to_string()));
/// }
/// ```
///
/// Messages can be efficiently read using a [`MessageReader`]:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(Message)]
/// # struct Greeting(String);
/// #
/// fn read_messages(mut reader: MessageReader<Greeting>) {
///     // Process all messages of type `Greeting`.
///     for Greeting(greeting) in reader.read() {
///         println!("{greeting}");
///     }
/// }
/// ```
/// [`Event`]: crate::event::Event
/// [`Observer`]: crate::observer::Observer
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an `Message`",
    label = "invalid `Message`",
    note = "consider annotating `{Self}` with `#[derive(Message)]`"
)]
pub trait Message: Send + Sync + 'static {}

#[derive(Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub(crate) struct MessageInstance<M: Message> {
    pub message_id: MessageId<M>,
    pub message: M,
}

/// A [`MessageId`] uniquely identifies a message stored in a specific [`World`].
///
/// A [`MessageId`] can, among other things, be used to trace the flow of a [`Message`] from the point it was
/// sent to the point it was processed. [`MessageId`]s increase monotonically by write order.
///
/// [`World`]: crate::world::World
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Clone, Debug, PartialEq, Hash)
)]
pub struct MessageId<M: Message> {
    /// Uniquely identifies the message associated with this ID.
    // This value corresponds to the order in which each message was written to the world.
    pub id: usize,
    /// The source code location that triggered this message.
    pub caller: MaybeLocation,
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore, clone))]
    pub(super) _marker: PhantomData<M>,
}

impl<M: Message> Copy for MessageId<M> {}

impl<M: Message> Clone for MessageId<M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M: Message> fmt::Display for MessageId<M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

impl<M: Message> fmt::Debug for MessageId<M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "message<{}>#{}",
            core::any::type_name::<M>().split("::").last().unwrap(),
            self.id,
        )
    }
}

impl<M: Message> PartialEq for MessageId<M> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<M: Message> Eq for MessageId<M> {}

impl<M: Message> PartialOrd for MessageId<M> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<M: Message> Ord for MessageId<M> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl<M: Message> Hash for MessageId<M> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self.id, state);
    }
}
