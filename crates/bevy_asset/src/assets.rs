use crate::{Asset, AssetEvent, AssetHandleProvider, AssetId, AssetServer, Handle};
use bevy_ecs::{
    prelude::EventWriter,
    system::{Res, ResMut, Resource},
};
use bevy_reflect::{Reflect, Uuid};
use bevy_utils::HashMap;
use crossbeam_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use std::{
    any::TypeId,
    iter::Enumerate,
    marker::PhantomData,
    sync::{atomic::AtomicU32, Arc},
};
use thiserror::Error;

/// A generational runtime-only identifier for a specific [`Asset`] stored in [`Assets`]. This is optimized for efficient runtime
/// usage and is not suitable for identifying assets across app runs.
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Reflect, Serialize, Deserialize,
)]
pub struct AssetIndex {
    pub(crate) generation: u32,
    pub(crate) index: u32,
}

/// Allocates generational [`AssetIndex`] values and facilitates their reuse.
pub(crate) struct AssetIndexAllocator {
    /// A monotonically increasing index.
    next_index: AtomicU32,
    recycled_queue_sender: Sender<AssetIndex>,
    /// This receives every recycled AssetIndex. It serves as a buffer/queue to store indices ready for reuse.
    recycled_queue_receiver: Receiver<AssetIndex>,
    recycled_sender: Sender<AssetIndex>,
    recycled_receiver: Receiver<AssetIndex>,
}

impl Default for AssetIndexAllocator {
    fn default() -> Self {
        let (recycled_queue_sender, recycled_queue_receiver) = crossbeam_channel::unbounded();
        let (recycled_sender, recycled_receiver) = crossbeam_channel::unbounded();
        Self {
            recycled_queue_sender,
            recycled_queue_receiver,
            recycled_sender,
            recycled_receiver,
            next_index: Default::default(),
        }
    }
}

impl AssetIndexAllocator {
    /// Reserves a new [`AssetIndex`], either by reusing a recycled index (with an incremented generation), or by creating a new index
    /// by incrementing the index counter for a given asset type `A`.
    pub fn reserve(&self) -> AssetIndex {
        if let Ok(mut recycled) = self.recycled_queue_receiver.try_recv() {
            recycled.generation += 1;
            self.recycled_sender.send(recycled).unwrap();
            recycled
        } else {
            AssetIndex {
                index: self
                    .next_index
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                generation: 0,
            }
        }
    }

    /// Queues the given `index` for reuse. This should only be done if the `index` is no longer being used.
    pub fn recycle(&self, index: AssetIndex) {
        self.recycled_queue_sender.send(index).unwrap();
    }
}

// PERF: do we actually need this to be an enum? Can we just use an "invalid" generation instead
#[derive(Default)]
enum Entry<A: Asset> {
    #[default]
    None,
    Some {
        value: Option<A>,
        generation: u32,
    },
}

/// Stores [`Asset`] values in a Vec-like storage identified by [`AssetIndex`].
struct DenseAssetStorage<A: Asset> {
    storage: Vec<Entry<A>>,
    len: u32,
    allocator: Arc<AssetIndexAllocator>,
}

impl<A: Asset> Default for DenseAssetStorage<A> {
    fn default() -> Self {
        Self {
            len: 0,
            storage: Default::default(),
            allocator: Default::default(),
        }
    }
}

impl<A: Asset> DenseAssetStorage<A> {
    // Returns the number of assets stored.
    pub(crate) fn len(&self) -> usize {
        self.len as usize
    }

    // Returns `true` if there are no assets stored.
    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Insert the value at the given index. Returns true if a value already exists (and was replaced)
    pub(crate) fn insert(
        &mut self,
        index: AssetIndex,
        asset: A,
    ) -> Result<bool, InvalidGenerationError> {
        self.flush();
        let entry = &mut self.storage[index.index as usize];
        if let Entry::Some { value, generation } = entry {
            if *generation == index.generation {
                let exists = value.is_some();
                if !exists {
                    self.len += 1;
                }
                *value = Some(asset);
                Ok(exists)
            } else {
                Err(InvalidGenerationError {
                    index,
                    current_generation: *generation,
                })
            }
        } else {
            unreachable!("entries should always be valid after a flush");
        }
    }

    /// Removes the asset stored at the given `index` and returns it as [`Some`] (if the asset exists).
    pub(crate) fn remove(&mut self, index: AssetIndex) -> Option<A> {
        self.flush();
        let value = match &mut self.storage[index.index as usize] {
            Entry::None => return None,
            Entry::Some { value, generation } => {
                if *generation == index.generation {
                    value.take().map(|value| {
                        self.len -= 1;
                        value
                    })
                } else {
                    return None;
                }
            }
        };
        self.storage[index.index as usize] = Entry::None;
        self.allocator.recycle(index);
        value
    }

    pub(crate) fn get(&self, index: AssetIndex) -> Option<&A> {
        let entry = self.storage.get(index.index as usize)?;
        match entry {
            Entry::None => None,
            Entry::Some { value, generation } => {
                if *generation == index.generation {
                    value.as_ref()
                } else {
                    None
                }
            }
        }
    }

    pub(crate) fn get_mut(&mut self, index: AssetIndex) -> Option<&mut A> {
        let entry = self.storage.get_mut(index.index as usize)?;
        match entry {
            Entry::None => None,
            Entry::Some { value, generation } => {
                if *generation == index.generation {
                    value.as_mut()
                } else {
                    None
                }
            }
        }
    }

    pub(crate) fn flush(&mut self) {
        // NOTE: this assumes the allocator index is monotonically increasing.
        let new_len = self
            .allocator
            .next_index
            .load(std::sync::atomic::Ordering::Relaxed);
        self.storage.resize_with(new_len as usize, || Entry::Some {
            value: None,
            generation: 0,
        });
        while let Ok(recycled) = self.allocator.recycled_receiver.try_recv() {
            let entry = &mut self.storage[recycled.index as usize];
            *entry = Entry::Some {
                value: None,
                generation: recycled.generation,
            };
        }
    }

    pub(crate) fn get_index_allocator(&self) -> Arc<AssetIndexAllocator> {
        self.allocator.clone()
    }

    pub(crate) fn ids(&self) -> impl Iterator<Item = AssetId<A>> + '_ {
        self.storage
            .iter()
            .enumerate()
            .filter_map(|(i, v)| match v {
                Entry::None => None,
                Entry::Some { value, generation } => {
                    if value.is_some() {
                        Some(AssetId::from(AssetIndex {
                            index: i as u32,
                            generation: *generation,
                        }))
                    } else {
                        None
                    }
                }
            })
    }
}

/// Stores [`Asset`] values identified by their [`AssetId`].
///
/// Assets identified by [`AssetId::Index`] will be stored in a "dense" vec-like storage. This is more efficient, but it means that
/// the assets can only be identified at runtime. This is the default behavior.
///
/// Assets identified by [`AssetId::Uuid`] will be stored in a hashmap. This is less efficient, but it means that the assets can be referenced
/// at compile time.
///
/// This tracks (and queues) [`AssetEvent`] events whenever changes to the collection occur.
#[derive(Resource)]
pub struct Assets<A: Asset> {
    dense_storage: DenseAssetStorage<A>,
    hash_map: HashMap<Uuid, A>,
    handle_provider: AssetHandleProvider,
    queued_events: Vec<AssetEvent<A>>,
}

impl<A: Asset> Default for Assets<A> {
    fn default() -> Self {
        let dense_storage = DenseAssetStorage::default();
        let handle_provider =
            AssetHandleProvider::new(TypeId::of::<A>(), dense_storage.get_index_allocator());
        Self {
            dense_storage,
            handle_provider,
            hash_map: Default::default(),
            queued_events: Default::default(),
        }
    }
}

impl<A: Asset> Assets<A> {
    /// Retrieves an [`AssetHandleProvider`] capable of reserving new [`Handle`] values for assets that will be stored in this
    /// collection.
    pub fn get_handle_provider(&self) -> AssetHandleProvider {
        self.handle_provider.clone()
    }

    /// Inserts the given `asset`, identified by the given `id`. If an asset already exists for `id`, it will be replaced.
    pub fn insert(&mut self, id: impl Into<AssetId<A>>, asset: A) {
        let id: AssetId<A> = id.into();
        match id {
            AssetId::Index { index, .. } => {
                self.insert_with_index(index, asset).unwrap();
            }
            AssetId::Uuid { uuid } => {
                self.insert_with_uuid(uuid, asset);
            }
        }
    }

    /// Retrieves an [`Asset`] stored for the given `id` if it exists. If it does not exist, it will be inserted using `insert_fn`.
    // PERF: Optimize this or remove it
    pub fn get_or_insert_with(
        &mut self,
        id: impl Into<AssetId<A>>,
        insert_fn: impl FnOnce() -> A,
    ) -> &mut A {
        let id: AssetId<A> = id.into();
        if self.get(id).is_none() {
            self.insert(id, (insert_fn)());
        }
        self.get_mut(id).unwrap()
    }

    /// Returns `true` if the `id` exists in this collection. Otherwise it returns `false`.
    // PERF: Optimize this or remove it
    pub fn contains(&self, id: impl Into<AssetId<A>>) -> bool {
        self.get(id).is_some()
    }

    pub(crate) fn insert_with_uuid(&mut self, uuid: Uuid, asset: A) -> Option<A> {
        let result = self.hash_map.insert(uuid, asset);
        if result.is_some() {
            self.queued_events
                .push(AssetEvent::Modified { id: uuid.into() });
        } else {
            self.queued_events
                .push(AssetEvent::Added { id: uuid.into() });
        }
        result
    }
    pub(crate) fn insert_with_index(
        &mut self,
        index: AssetIndex,
        asset: A,
    ) -> Result<bool, InvalidGenerationError> {
        let replaced = self.dense_storage.insert(index, asset)?;
        if replaced {
            self.queued_events
                .push(AssetEvent::Modified { id: index.into() });
        } else {
            self.queued_events
                .push(AssetEvent::Added { id: index.into() });
        }
        Ok(replaced)
    }

    /// Adds the given `asset` and allocates a new strong [`Handle`] for it.
    #[inline]
    pub fn add(&mut self, asset: A) -> Handle<A> {
        let index = self.dense_storage.allocator.reserve();
        self.insert_with_index(index, asset).unwrap();
        Handle::Strong(
            self.handle_provider
                .get_handle(index.into(), false, None, None),
        )
    }

    /// Retrieves a reference to the [`Asset`] with the given `id`, if its exists.
    /// Note that this supports anything that implements `Into<AssetId<A>>`, which includes [`Handle`] and [`AssetId`].
    #[inline]
    pub fn get(&self, id: impl Into<AssetId<A>>) -> Option<&A> {
        let id: AssetId<A> = id.into();
        match id {
            AssetId::Index { index, .. } => self.dense_storage.get(index),
            AssetId::Uuid { uuid } => self.hash_map.get(&uuid),
        }
    }

    /// Retrieves a mutable reference to the [`Asset`] with the given `id`, if its exists.
    /// Note that this supports anything that implements `Into<AssetId<A>>`, which includes [`Handle`] and [`AssetId`].
    #[inline]
    pub fn get_mut(&mut self, id: impl Into<AssetId<A>>) -> Option<&mut A> {
        let id: AssetId<A> = id.into();
        let result = match id {
            AssetId::Index { index, .. } => self.dense_storage.get_mut(index),
            AssetId::Uuid { uuid } => self.hash_map.get_mut(&uuid),
        };
        if result.is_some() {
            self.queued_events.push(AssetEvent::Modified { id });
        }
        result
    }

    /// Removes (and returns) the [`Asset`] with the given `id`, if its exists.
    /// Note that this supports anything that implements `Into<AssetId<A>>`, which includes [`Handle`] and [`AssetId`].
    pub fn remove(&mut self, id: impl Into<AssetId<A>>) -> Option<A> {
        let id: AssetId<A> = id.into();
        let result = self.remove_untracked(id);
        if result.is_some() {
            self.queued_events.push(AssetEvent::Removed { id });
        }
        result
    }

    /// Removes (and returns) the [`Asset`] with the given `id`, if its exists. This skips emitting [`AssetEvent::Removed`].
    /// Note that this supports anything that implements `Into<AssetId<A>>`, which includes [`Handle`] and [`AssetId`].
    pub fn remove_untracked(&mut self, id: impl Into<AssetId<A>>) -> Option<A> {
        let id: AssetId<A> = id.into();
        match id {
            AssetId::Index { index, .. } => self.dense_storage.remove(index),
            AssetId::Uuid { uuid } => self.hash_map.remove(&uuid),
        }
    }

    /// Returns `true` if there are no assets in this collection.
    pub fn is_empty(&self) -> bool {
        self.dense_storage.is_empty() && self.hash_map.is_empty()
    }

    /// Returns the number of assets currently stored in the collection.
    pub fn len(&self) -> usize {
        self.dense_storage.len() + self.hash_map.len()
    }

    /// Returns an iterator over the [`AssetId`] of every [`Asset`] stored in this collection.
    pub fn ids(&self) -> impl Iterator<Item = AssetId<A>> + '_ {
        self.dense_storage
            .ids()
            .chain(self.hash_map.keys().map(|uuid| AssetId::from(*uuid)))
    }

    /// Returns an iterator over the [`AssetId`] and [`Asset`] ref of every asset in this collection.
    // PERF: this could be accelerated if we implement a skip list. Consider the cost/benefits
    pub fn iter(&self) -> impl Iterator<Item = (AssetId<A>, &A)> {
        self.dense_storage
            .storage
            .iter()
            .enumerate()
            .filter_map(|(i, v)| match v {
                Entry::None => None,
                Entry::Some { value, generation } => value.as_ref().map(|v| {
                    let id = AssetId::Index {
                        index: AssetIndex {
                            generation: *generation,
                            index: i as u32,
                        },
                        marker: PhantomData,
                    };
                    (id, v)
                }),
            })
            .chain(
                self.hash_map
                    .iter()
                    .map(|(i, v)| (AssetId::Uuid { uuid: *i }, v)),
            )
    }

    /// Returns an iterator over the [`AssetId`] and mutable [`Asset`] ref of every asset in this collection.
    // PERF: this could be accelerated if we implement a skip list. Consider the cost/benefits
    pub fn iter_mut(&mut self) -> AssetsMutIterator<'_, A> {
        AssetsMutIterator {
            dense_storage: self.dense_storage.storage.iter_mut().enumerate(),
            hash_map: self.hash_map.iter_mut(),
            queued_events: &mut self.queued_events,
        }
    }

    /// A system that synchronizes the state of assets in this collection with the [`AssetServer`]. This manages
    /// [`Handle`] drop events and adds queued [`AssetEvent`] values to their [`Events`] resource.
    ///
    /// [`Events`]: bevy_ecs::event::Events
    pub fn track_assets(mut assets: ResMut<Self>, asset_server: Res<AssetServer>) {
        let assets = &mut *assets;
        // note that we must hold this lock for the entire duration of this function to ensure
        // that `asset_server.load` calls that occur during it block, which ensures that
        // re-loads are kicked off appropriately. This function must be "transactional" relative
        // to other asset info operations
        let mut infos = asset_server.data.infos.write();
        let mut not_ready = Vec::new();
        while let Ok(drop_event) = assets.handle_provider.drop_receiver.try_recv() {
            let id = drop_event.id;
            if !assets.contains(id.typed()) {
                not_ready.push(drop_event);
                continue;
            }
            if drop_event.asset_server_managed {
                if infos.process_handle_drop(id.untyped(TypeId::of::<A>())) {
                    assets.remove(id.typed());
                }
            } else {
                assets.remove(id.typed());
            }
        }
        // TODO: this is _extremely_ inefficient find a better fix
        // This will also loop failed assets indefinitely. Is that ok?
        for event in not_ready {
            assets.handle_provider.drop_sender.send(event).unwrap();
        }
    }

    /// A system that applies accumulated asset change events to the [`Events`] resource.
    ///
    /// [`Events`]: bevy_ecs::event::Events
    pub fn asset_events(mut assets: ResMut<Self>, mut events: EventWriter<AssetEvent<A>>) {
        events.send_batch(assets.queued_events.drain(..));
    }
}

/// A mutable iterator over [`Assets`].
pub struct AssetsMutIterator<'a, A: Asset> {
    queued_events: &'a mut Vec<AssetEvent<A>>,
    dense_storage: Enumerate<std::slice::IterMut<'a, Entry<A>>>,
    hash_map: bevy_utils::hashbrown::hash_map::IterMut<'a, Uuid, A>,
}

impl<'a, A: Asset> Iterator for AssetsMutIterator<'a, A> {
    type Item = (AssetId<A>, &'a mut A);

    fn next(&mut self) -> Option<Self::Item> {
        for (i, entry) in &mut self.dense_storage {
            match entry {
                Entry::None => {
                    continue;
                }
                Entry::Some { value, generation } => {
                    let id = AssetId::Index {
                        index: AssetIndex {
                            generation: *generation,
                            index: i as u32,
                        },
                        marker: PhantomData,
                    };
                    self.queued_events.push(AssetEvent::Modified { id });
                    if let Some(value) = value {
                        return Some((id, value));
                    }
                }
            }
        }
        if let Some((key, value)) = self.hash_map.next() {
            let id = AssetId::Uuid { uuid: *key };
            self.queued_events.push(AssetEvent::Modified { id });
            Some((id, value))
        } else {
            None
        }
    }
}

#[derive(Error, Debug)]
#[error("AssetIndex {index:?} has an invalid generation. The current generation is: '{current_generation}'.")]
pub struct InvalidGenerationError {
    index: AssetIndex,
    current_generation: u32,
}
