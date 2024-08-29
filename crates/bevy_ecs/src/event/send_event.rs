use super::{Event, Events};
use crate::world::{Command, World};

/// A command to send an arbitrary [`Event`], used by [`Commands::send_event`](crate::system::Commands::send_event).
pub struct SendEvent<E: Event> {
    /// The event to send.
    pub event: E,
}

impl<E: Event> Command for SendEvent<E> {
    fn apply(self, world: &mut World) {
        let mut events = world.resource_mut::<Events<E>>();
        events.send(self.event);
    }
}
