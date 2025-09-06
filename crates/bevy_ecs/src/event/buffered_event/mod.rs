mod collections;
mod event_cursor;
mod iterators;
mod mut_iterators;
mod mutator;
mod reader;
mod registry;
mod update;
mod writer;

pub use collections::*;
pub use event_cursor::*;
pub use iterators::*;
pub use mut_iterators::*;
pub use mutator::*;
pub use reader::*;
pub use registry::*;
pub use update::*;
pub use writer::*;

use crate::change_detection::MaybeLocation;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use core::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

/// A buffered event for pull-based event handling.
///
/// Buffered events can be written with [`EventWriter`] and read using the [`EventReader`] system parameter.
/// These events are stored in the [`Events<E>`] resource, and require periodically polling the world for new events,
/// typically in a system that runs as part of a schedule.
///
/// While the polling imposes a small overhead, buffered events are useful for efficiently batch processing
/// a large number of events at once. This can make them more efficient than [`Event`]s used by [`Observer`]s
/// for events that happen at a high frequency or in large quantities.
///
/// Unlike [`Event`]s triggered for observers, buffered events are evaluated at fixed points in the schedule
/// rather than immediately when they are sent. This allows for more predictable scheduling and deferring
/// event processing to a later point in time.
///
/// Buffered events do *not* trigger observers automatically when they are written via an [`EventWriter`].
/// However, they can still also be triggered on a [`World`] in case you want both buffered and immediate
/// event handling for the same event.
///
/// Buffered events must be thread-safe.
///
/// # Usage
///
/// The [`BufferedEvent`] trait can be derived:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// #[derive(BufferedEvent)]
/// struct Message(String);
/// ```
///
/// The event can then be written to the event buffer using an [`EventWriter`]:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(BufferedEvent)]
/// # struct Message(String);
/// #
/// fn write_hello(mut writer: EventWriter<Message>) {
///     writer.write(Message("Hello!".to_string()));
/// }
/// ```
///
/// Buffered events can be efficiently read using an [`EventReader`]:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(BufferedEvent)]
/// # struct Message(String);
/// #
/// fn read_messages(mut reader: EventReader<Message>) {
///     // Process all buffered events of type `Message`.
///     for Message(message) in reader.read() {
///         println!("{message}");
///     }
/// }
/// ```
/// [`Event`]: crate::event::Event
/// [`World`]: crate::world::World
/// [`Observer`]: crate::observer::Observer
/// [`Events<E>`]: super::Events
/// [`EventReader`]: super::EventReader
/// [`EventWriter`]: super::EventWriter
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an `BufferedEvent`",
    label = "invalid `BufferedEvent`",
    note = "consider annotating `{Self}` with `#[derive(BufferedEvent)]`"
)]
pub trait BufferedEvent: Send + Sync + 'static {}

#[derive(Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub(crate) struct EventInstance<E: BufferedEvent> {
    pub event_id: EventId<E>,
    pub event: E,
}

/// An `EventId` uniquely identifies an event stored in a specific [`World`].
///
/// An `EventId` can among other things be used to trace the flow of an event from the point it was
/// sent to the point it was processed. `EventId`s increase monotonically by send order.
///
/// [`World`]: crate::world::World
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Clone, Debug, PartialEq, Hash)
)]
pub struct EventId<E: BufferedEvent> {
    /// Uniquely identifies the event associated with this ID.
    // This value corresponds to the order in which each event was added to the world.
    pub id: usize,
    /// The source code location that triggered this event.
    pub caller: MaybeLocation,
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore, clone))]
    pub(super) _marker: PhantomData<E>,
}

impl<E: BufferedEvent> Copy for EventId<E> {}

impl<E: BufferedEvent> Clone for EventId<E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<E: BufferedEvent> fmt::Display for EventId<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

impl<E: BufferedEvent> fmt::Debug for EventId<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "event<{}>#{}",
            core::any::type_name::<E>().split("::").last().unwrap(),
            self.id,
        )
    }
}

impl<E: BufferedEvent> PartialEq for EventId<E> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<E: BufferedEvent> Eq for EventId<E> {}

impl<E: BufferedEvent> PartialOrd for EventId<E> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<E: BufferedEvent> Ord for EventId<E> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl<E: BufferedEvent> Hash for EventId<E> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self.id, state);
    }
}
