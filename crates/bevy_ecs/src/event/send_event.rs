#[cfg(feature = "track_change_detection")]
use core::panic::Location;

use super::{Event, Events};
use crate::world::{Command, World};

/// A command to send an arbitrary [`Event`], used by [`Commands::send_event`](crate::system::Commands::send_event).
pub struct SendEvent<E: Event> {
    /// The event to send.
    pub event: E,
    /// The source code location that triggered this command.
    #[cfg(feature = "track_change_detection")]
    pub caller: &'static Location<'static>,
}

// This does not use `From`, as the resulting `Into` is not track_caller
impl<E: Event> SendEvent<E> {
    /// Constructs a new `SendEvent` tracking the caller.
    pub fn new(event: E) -> Self {
        Self {
            event,
            #[cfg(feature = "track_change_detection")]
            caller: Location::caller(),
        }
    }
}

impl<E: Event> Command for SendEvent<E> {
    fn apply(self, world: &mut World) {
        let mut events = world.resource_mut::<Events<E>>();
        events.send_with_caller(
            self.event,
            #[cfg(feature = "track_change_detection")]
            self.caller,
        );
    }
}
