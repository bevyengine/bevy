use bevy_ecs::{
    event::{BufferedEvent, EventId, Events, WriteBatchIds},
    system::{ResMut, SystemParam},
};

/// Writes [`BufferedEvent`]s of type `T`.
///
/// # Usage
///
/// `EventWriter`s are usually declared as a [`SystemParam`].
/// ```
/// # use bevy_ecs::prelude::*;
///
/// #[derive(Event, BufferedEvent)]
/// pub struct MyEvent; // Custom event type.
/// fn my_system(mut writer: EventWriter<MyEvent>) {
///     writer.write(MyEvent);
/// }
///
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
/// # Observers
///
/// "Buffered" events, such as those sent directly in [`Events`] or written using [`EventWriter`], do _not_ automatically
/// trigger any [`Observer`]s watching for that event, as each [`BufferedEvent`] has different requirements regarding _if_ it will
/// be triggered, and if so, _when_ it will be triggered in the schedule.
///
/// # Concurrency
///
/// `EventWriter` param has [`ResMut<Events<T>>`](Events) inside. So two systems declaring `EventWriter<T>` params
/// for the same event type won't be executed concurrently.
///
/// # Untyped events
///
/// `EventWriter` can only write events of one specific type, which must be known at compile-time.
/// This is not a problem most of the time, but you may find a situation where you cannot know
/// ahead of time every kind of event you'll need to write. In this case, you can use the "type-erased event" pattern.
///
/// ```
/// # use bevy_ecs::{prelude::*, event::Events};
/// # #[derive(Event, BufferedEvent)]
/// # pub struct MyEvent;
/// fn write_untyped(mut commands: Commands) {
///     // Write an event of a specific type without having to declare that
///     // type as a SystemParam.
///     //
///     // Effectively, we're just moving the type parameter from the /type/ to the /method/,
///     // which allows one to do all kinds of clever things with type erasure, such as sending
///     // custom events to unknown 3rd party plugins (modding API).
///     //
///     // NOTE: the event won't actually be sent until commands get applied during
///     // apply_deferred.
///     commands.queue(|w: &mut World| {
///         w.write_event(MyEvent);
///     });
/// }
/// ```
/// Note that this is considered *non-idiomatic*, and should only be used when `EventWriter` will not work.
///
/// [`Observer`]: crate::observer::Observer
#[derive(SystemParam)]
pub struct EventWriter<'w, E: BufferedEvent> {
    #[system_param(validation_message = "BufferedEvent not initialized")]
    events: ResMut<'w, Events<E>>,
}

impl<'w, E: BufferedEvent> EventWriter<'w, E> {
    /// Writes an `event`, which can later be read by [`EventReader`](super::EventReader)s.
    /// This method returns the [ID](`EventId`) of the written `event`.
    ///
    /// See [`Events`] for details.
    #[doc(alias = "send")]
    #[track_caller]
    pub fn write(&mut self, event: E) -> EventId<E> {
        self.events.write(event)
    }

    /// Writes a list of `events` all at once, which can later be read by [`EventReader`](super::EventReader)s.
    /// This is more efficient than writing each event individually.
    /// This method returns the [IDs](`EventId`) of the written `events`.
    ///
    /// See [`Events`] for details.
    #[doc(alias = "send_batch")]
    #[track_caller]
    pub fn write_batch(&mut self, events: impl IntoIterator<Item = E>) -> WriteBatchIds<E> {
        self.events.write_batch(events)
    }

    /// Writes the default value of the event. Useful when the event is an empty struct.
    /// This method returns the [ID](`EventId`) of the written `event`.
    ///
    /// See [`Events`] for details.
    #[doc(alias = "send_default")]
    #[track_caller]
    pub fn write_default(&mut self) -> EventId<E>
    where
        E: Default,
    {
        self.events.write_default()
    }
}
