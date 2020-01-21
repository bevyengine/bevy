use crate::entity::Entity;
use crate::filter::{
    ArchetypeFilterData, ChunkFilterData, ChunksetFilterData, EntityFilter, Filter, FilterResult,
};
use crate::storage::ArchetypeId;
use crate::storage::ChunkId;
use crossbeam_channel::{Sender, TrySendError};
use std::sync::Arc;

/// Events emitted by a world to subscribers. See `World.subscribe(Sender, EntityFilter)`.
#[derive(Debug, Clone)]
pub enum Event {
    /// A new archetype has been created.
    ArchetypeCreated(ArchetypeId),
    /// A new chunk has been created.
    ChunkCreated(ChunkId),
    /// An entity has been inserted into a chunk.
    EntityInserted(Entity, ChunkId),
    /// An entity has been removed from a chunk.
    EntityRemoved(Entity, ChunkId),
}

pub(crate) trait EventFilter: Send + Sync + 'static {
    fn matches_archetype(&self, data: ArchetypeFilterData, index: usize) -> bool;
    fn matches_chunkset(&self, data: ChunksetFilterData, index: usize) -> bool;
    fn matches_chunk(&self, data: ChunkFilterData, index: usize) -> bool;
}

pub(crate) struct EventFilterWrapper<T: EntityFilter + Sync + 'static>(pub T);

impl<T: EntityFilter + Sync + 'static> EventFilter for EventFilterWrapper<T> {
    fn matches_archetype(&self, data: ArchetypeFilterData, index: usize) -> bool {
        let (filter, _, _) = self.0.filters();
        if let Some(element) = filter.collect(data).nth(index) {
            return filter.is_match(&element).is_pass();
        }

        false
    }

    fn matches_chunkset(&self, data: ChunksetFilterData, index: usize) -> bool {
        let (_, filter, _) = self.0.filters();
        if let Some(element) = filter.collect(data).nth(index) {
            return filter.is_match(&element).is_pass();
        }

        false
    }

    fn matches_chunk(&self, data: ChunkFilterData, index: usize) -> bool {
        let (_, _, filter) = self.0.filters();
        if let Some(element) = filter.collect(data).nth(index) {
            return filter.is_match(&element).is_pass();
        }

        false
    }
}

#[derive(Clone)]
pub(crate) struct Subscriber {
    pub filter: Arc<dyn EventFilter>,
    pub sender: Sender<Event>,
}

impl Subscriber {
    pub fn new(filter: Arc<dyn EventFilter>, sender: Sender<Event>) -> Self {
        Self { filter, sender }
    }
}

#[derive(Clone)]
pub(crate) struct Subscribers {
    subscribers: Vec<Subscriber>,
}

impl Subscribers {
    pub fn new() -> Self {
        Self {
            subscribers: Vec::new(),
        }
    }

    pub fn push(&mut self, subscriber: Subscriber) { self.subscribers.push(subscriber); }

    pub fn send(&mut self, message: Event) {
        for i in (0..self.subscribers.len()).rev() {
            if let Err(error) = self.subscribers[i].sender.try_send(message.clone()) {
                if let TrySendError::Disconnected(_) = error {
                    self.subscribers.swap_remove(i);
                }
            }
        }
    }

    pub fn matches_archetype(&self, data: ArchetypeFilterData, index: usize) -> Self {
        let subscribers = self
            .subscribers
            .iter()
            .filter(|sub| sub.filter.matches_archetype(data, index))
            .cloned()
            .collect();
        Self { subscribers }
    }

    pub fn matches_chunkset(&self, data: ChunksetFilterData, index: usize) -> Self {
        let subscribers = self
            .subscribers
            .iter()
            .filter(|sub| sub.filter.matches_chunkset(data, index))
            .cloned()
            .collect();
        Self { subscribers }
    }

    pub fn matches_chunk(&self, data: ChunkFilterData, index: usize) -> Self {
        let subscribers = self
            .subscribers
            .iter()
            .filter(|sub| sub.filter.matches_chunk(data, index))
            .cloned()
            .collect();
        Self { subscribers }
    }
}

impl Default for Subscribers {
    fn default() -> Self { Subscribers::new() }
}
