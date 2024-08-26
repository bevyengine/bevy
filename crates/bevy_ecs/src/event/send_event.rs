use crate::{
    system::Commands,
    world::{Command, World},
};

use super::{Event, Events};

struct FireEvent<E: Event> {
    event: E,
}

impl<E: Event> Command for FireEvent<E> {
    fn apply(self, world: &mut World) {
        let mut events = world.resource_mut::<Events<E>>();
        events.send(self.event);
    }
}

/// Send an arbitrary event via commands
pub trait SendEventEx {
    /// Send an arbitrary event via commands
    fn send_event<E: Event>(&mut self, e: E) -> &mut Self;
}

impl SendEventEx for Commands<'_, '_> {
    fn send_event<E: Event>(&mut self, event: E) -> &mut Self {
        self.add(FireEvent { event });
        self
    }
}
