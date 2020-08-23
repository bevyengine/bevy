use super::listener::Listener;
use super::common::ListenerId;

use std::collections::HashMap;

#[derive(Default)]
pub struct Listeners {
    listeners: HashMap<ListenerId, Box<dyn Listener>>,
}

impl Listeners {
    pub fn add(&mut self, listener: Box<dyn Listener>) {
        self.listeners.insert(listener.get_id(), listener);
    }

    pub fn get(&self, id: ListenerId) -> Option<&Box<dyn Listener>> {
        self.listeners.get(&id)
    }

    pub fn get_mut(&mut self, id: ListenerId) -> Option<&mut Box<dyn Listener>> {
        self.listeners.get_mut(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Box<dyn Listener>> {
        self.listeners.values()
    }
}
