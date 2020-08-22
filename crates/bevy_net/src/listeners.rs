use super::listener::Listener;
use super::common::ListenerId;

use std::collections::HashMap;

#[derive(Default)]
pub struct Listeners {
    listeners: HashMap<ListenerId, Listener>,
}

impl Listeners {
    pub fn add(&mut self, listener: Listener) {
        self.listeners.insert(listener.id, listener);
    }

    pub fn get(&self, id: ListenerId) -> Option<&Listener> {
        self.listeners.get(&id)
    }

    pub fn get_mut(&mut self, id: ListenerId) -> Option<&mut Listener> {
        self.listeners.get_mut(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Listener> {
        self.listeners.values()
    }
}
