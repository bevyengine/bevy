use crate as bevy_ecs;
use bevy_ecs::{
    event::{Event, EventId, Events, SendBatchIds},
    system::{ResMut, SystemParam},
};

/// Sends events of type `T`.
///
/// # Usage
///
/// `EventWriter`s are usually declared as a [`SystemParam`].
/// ```
/// # use bevy_ecs::prelude::*;
///
/// #[derive(Event)]
/// pub struct MyEvent; // Custom event type.
/// fn my_system(mut writer: EventWriter<MyEvent>) {
///     writer.send(MyEvent);
/// }
///
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
/// # Observers
///
/// "Buffered" Events, such as those sent directly in [`Events`] or sent using [`EventWriter`], do _not_ automatically
/// trigger any [`Observer`]s watching for that event, as each [`Event`] has different requirements regarding _if_ it will
/// be triggered, and if so, _when_ it will be triggered in the schedule.
///
/// # Concurrency
///
/// `EventWriter` param has [`ResMut<Events<T>>`](Events) inside. So two systems declaring `EventWriter<T>` params
/// for the same event type won't be executed concurrently.
///
/// # Untyped events
///
/// `EventWriter` can only send events of one specific type, which must be known at compile-time.
/// This is not a problem most of the time, but you may find a situation where you cannot know
/// ahead of time every kind of event you'll need to send. In this case, you can use the "type-erased event" pattern.
///
/// ```
/// # use bevy_ecs::{prelude::*, event::Events};
/// # #[derive(Event)]
/// # pub struct MyEvent;
/// fn send_untyped(mut commands: Commands) {
///     // Send an event of a specific type without having to declare that
///     // type as a SystemParam.
///     //
///     // Effectively, we're just moving the type parameter from the /type/ to the /method/,
///     // which allows one to do all kinds of clever things with type erasure, such as sending
///     // custom events to unknown 3rd party plugins (modding API).
///     //
///     // NOTE: the event won't actually be sent until commands get applied during
///     // apply_deferred.
///     commands.add(|w: &mut World| {
///         w.send_event(MyEvent);
///     });
/// }
/// ```
/// Note that this is considered *non-idiomatic*, and should only be used when `EventWriter` will not work.
///
/// [`Observer`]: crate::observer::Observer
#[derive(SystemParam)]
pub struct EventWriter<'w, E: Event> {
    events: ResMut<'w, Events<E>>,
}

impl<'w, E: Event> EventWriter<'w, E> {
    /// Sends an `event`, which can later be read by [`EventReader`]s.
    /// This method returns the [ID](`EventId`) of the sent `event`.
    ///
    /// See [`Events`] for details.
    pub fn send(&mut self, event: E) -> EventId<E> {
        self.events.send(event)
    }

    /// Sends a list of `events` all at once, which can later be read by [`EventReader`]s.
    /// This is more efficient than sending each event individually.
    /// This method returns the [IDs](`EventId`) of the sent `events`.
    ///
    /// See [`Events`] for details.
    pub fn send_batch(&mut self, events: impl IntoIterator<Item = E>) -> SendBatchIds<E> {
        self.events.send_batch(events)
    }

    /// Sends the default value of the event. Useful when the event is an empty struct.
    /// This method returns the [ID](`EventId`) of the sent `event`.
    ///
    /// See [`Events`] for details.
    pub fn send_default(&mut self) -> EventId<E>
    where
        E: Default,
    {
        self.events.send_default()
    }
}
