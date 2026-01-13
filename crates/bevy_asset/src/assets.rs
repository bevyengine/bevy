use crate::asset_changed::AssetChanges;
use crate::{Asset, AssetEvent, AssetHandleProvider, AssetId, AssetServer, Handle, UntypedHandle};
use alloc::{sync::Arc, vec::Vec};
use bevy_ecs::{
    message::MessageWriter,
    resource::Resource,
    system::{Res, ResMut, SystemChangeTick},
};
use bevy_platform::collections::HashMap;
use bevy_reflect::{Reflect, TypePath};
use core::ops::{Deref, DerefMut};
use core::{any::TypeId, iter::Enumerate, marker::PhantomData, sync::atomic::AtomicU32};
use crossbeam_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// A generational runtime-only identifier for a specific [`Asset`] stored in [`Assets`]. This is optimized for efficient runtime
/// usage and is not suitable for identifying assets across app runs.
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Reflect, Serialize, Deserialize,
)]
pub struct AssetIndex {
    pub(crate) generation: u32,
    pub(crate) index: u32,
}

impl AssetIndex {
    /// Convert the [`AssetIndex`] into an opaque blob of bits to transport it in circumstances where carrying a strongly typed index isn't possible.
    ///
    /// The result of this function should not be relied upon for anything except putting it back into [`AssetIndex::from_bits`] to recover the index.
    pub fn to_bits(self) -> u64 {
        let Self { generation, index } = self;
        ((generation as u64) << 32) | index as u64
    }
    /// Convert an opaque `u64` acquired from [`AssetIndex::to_bits`] back into an [`AssetIndex`]. This should not be used with any inputs other than those
    /// derived from [`AssetIndex::to_bits`], as there are no guarantees for what will happen with such inputs.
    pub fn from_bits(bits: u64) -> Self {
        let index = ((bits << 32) >> 32) as u32;
        let generation = (bits >> 32) as u32;
        Self { generation, index }
    }
}

/// Allocates generational [`AssetIndex`] values and facilitates their reuse.
pub(crate) struct AssetIndexAllocator {
    /// A monotonically increasing index.
    next_index: AtomicU32,
    recycled_queue_sender: Sender<AssetIndex>,
    /// This receives every recycled [`AssetIndex`]. It serves as a buffer/queue to store indices ready for reuse.
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
                    .fetch_add(1, core::sync::atomic::Ordering::Relaxed),
                generation: 0,
            }
        }
    }

    /// Queues the given `index` for reuse. This should only be done if the `index` is no longer being used.
    pub fn recycle(&self, index: AssetIndex) {
        self.recycled_queue_sender.send(index).unwrap();
    }
}

/// A "loaded asset" containing the untyped handle for an asset stored in a given [`AssetPath`].
///
/// [`AssetPath`]: crate::AssetPath
#[derive(Asset, TypePath)]
pub struct LoadedUntypedAsset {
    /// The handle to the loaded asset.
    #[dependency]
    pub handle: UntypedHandle,
}

// PERF: do we actually need this to be an enum? Can we just use an "invalid" generation instead
#[derive(Default)]
enum Entry<A: Asset> {
    /// None is an indicator that this entry does not have live handles.
    #[default]
    None,
    /// Some is an indicator that there is a live handle active for the entry at this [`AssetIndex`]
    Some { value: Option<A>, generation: u32 },
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
                Err(InvalidGenerationError::Occupied {
                    index,
                    current_generation: *generation,
                })
            }
        } else {
            Err(InvalidGenerationError::Removed { index })
        }
    }

    /// Removes the asset stored at the given `index` and returns it as [`Some`] (if the asset exists).
    /// This will recycle the id and allow new entries to be inserted.
    pub(crate) fn remove_dropped(&mut self, index: AssetIndex) -> Option<A> {
        self.remove_internal(index, |dense_storage| {
            dense_storage.storage[index.index as usize] = Entry::None;
            dense_storage.allocator.recycle(index);
        })
    }

    /// Removes the asset stored at the given `index` and returns it as [`Some`] (if the asset exists).
    /// This will _not_ recycle the id. New values with the current ID can still be inserted. The ID will
    /// not be reused until [`DenseAssetStorage::remove_dropped`] is called.
    pub(crate) fn remove_still_alive(&mut self, index: AssetIndex) -> Option<A> {
        self.remove_internal(index, |_| {})
    }

    fn remove_internal(
        &mut self,
        index: AssetIndex,
        removed_action: impl FnOnce(&mut Self),
    ) -> Option<A> {
        self.flush();
        let value = match &mut self.storage[index.index as usize] {
            Entry::None => return None,
            Entry::Some { value, generation } => {
                if *generation == index.generation {
                    value.take().inspect(|_| self.len -= 1)
                } else {
                    return None;
                }
            }
        };
        removed_action(self);
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
            .load(core::sync::atomic::Ordering::Relaxed);
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
/// To check whether the asset used by a given component has changed (due to a change in the handle or the underlying asset)
/// use the [`AssetChanged`](crate::asset_changed::AssetChanged) query filter.
#[derive(Resource)]
pub struct Assets<A: Asset> {
    dense_storage: DenseAssetStorage<A>,
    hash_map: HashMap<Uuid, A>,
    handle_provider: AssetHandleProvider,
    queued_events: Vec<AssetEvent<A>>,
    /// Assets managed by the `Assets` struct with live strong `Handle`s
    /// originating from `get_strong_handle`.
    duplicate_handles: HashMap<AssetIndex, u16>,
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
            duplicate_handles: Default::default(),
        }
    }
}

impl<A: Asset> Assets<A> {
    /// Retrieves an [`AssetHandleProvider`] capable of reserving new [`Handle`] values for assets that will be stored in this
    /// collection.
    pub fn get_handle_provider(&self) -> AssetHandleProvider {
        self.handle_provider.clone()
    }

    /// Reserves a new [`Handle`] for an asset that will be stored in this collection.
    pub fn reserve_handle(&self) -> Handle<A> {
        self.handle_provider.reserve_handle().typed::<A>()
    }

    /// Inserts the given `asset`, identified by the given `id`. If an asset already exists for
    /// `id`, it will be replaced.
    ///
    /// Note: This will never return an error for UUID asset IDs.
    pub fn insert(
        &mut self,
        id: impl Into<AssetId<A>>,
        asset: A,
    ) -> Result<(), InvalidGenerationError> {
        match id.into() {
            AssetId::Index { index, .. } => self.insert_with_index(index, asset).map(|_| ()),
            AssetId::Uuid { uuid } => {
                self.insert_with_uuid(uuid, asset);
                Ok(())
            }
        }
    }

    /// Retrieves an [`Asset`] stored for the given `id` if it exists. If it does not exist, it will
    /// be inserted using `insert_fn`.
    ///
    /// Note: This will never return an error for UUID asset IDs.
    // PERF: Optimize this or remove it
    pub fn get_or_insert_with(
        &mut self,
        id: impl Into<AssetId<A>>,
        insert_fn: impl FnOnce() -> A,
    ) -> Result<AssetMut<'_, A>, InvalidGenerationError> {
        let id: AssetId<A> = id.into();
        if self.get(id).is_none() {
            self.insert(id, insert_fn())?;
        }
        // This should be impossible since either, `self.get` was Some, in which case this succeeds,
        // or `self.get` was None and we inserted it (and bailed out if there was an error).
        Ok(self
            .get_mut(id)
            .expect("the Asset was none even though we checked or inserted"))
    }

    /// Returns `true` if the `id` exists in this collection. Otherwise it returns `false`.
    pub fn contains(&self, id: impl Into<AssetId<A>>) -> bool {
        match id.into() {
            AssetId::Index { index, .. } => self.dense_storage.get(index).is_some(),
            AssetId::Uuid { uuid } => self.hash_map.contains_key(&uuid),
        }
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
    pub fn add(&mut self, asset: impl Into<A>) -> Handle<A> {
        let index = self.dense_storage.allocator.reserve();
        self.insert_with_index(index, asset.into()).unwrap();
        Handle::Strong(self.handle_provider.get_handle(index, false, None, None))
    }

    /// Upgrade an `AssetId` into a strong `Handle` that will prevent asset drop.
    ///
    /// Returns `None` if the provided `id` is not part of this `Assets` collection.
    /// For example, it may have been dropped earlier.
    #[inline]
    pub fn get_strong_handle(&mut self, id: AssetId<A>) -> Option<Handle<A>> {
        if !self.contains(id) {
            return None;
        }
        let index = match id {
            AssetId::Index { index, .. } => index,
            // We don't support strong handles for Uuid assets.
            AssetId::Uuid { .. } => return None,
        };
        *self.duplicate_handles.entry(index).or_insert(0) += 1;
        Some(Handle::Strong(
            self.handle_provider.get_handle(index, false, None, None),
        ))
    }

    /// Retrieves a reference to the [`Asset`] with the given `id`, if it exists.
    /// Note that this supports anything that implements `Into<AssetId<A>>`, which includes [`Handle`] and [`AssetId`].
    #[inline]
    pub fn get(&self, id: impl Into<AssetId<A>>) -> Option<&A> {
        match id.into() {
            AssetId::Index { index, .. } => self.dense_storage.get(index),
            AssetId::Uuid { uuid } => self.hash_map.get(&uuid),
        }
    }

    /// Retrieves a mutable reference to the [`Asset`] with the given `id`, if it exists.
    /// Note that this supports anything that implements `Into<AssetId<A>>`, which includes [`Handle`] and [`AssetId`].
    #[inline]
    pub fn get_mut(&mut self, id: impl Into<AssetId<A>>) -> Option<AssetMut<'_, A>> {
        let id: AssetId<A> = id.into();
        let result = match id {
            AssetId::Index { index, .. } => self.dense_storage.get_mut(index),
            AssetId::Uuid { uuid } => self.hash_map.get_mut(&uuid),
        };
        Some(AssetMut {
            asset: result?,
            guard: AssetMutChangeNotifier {
                changed: false,
                asset_id: id,
                queued_events: &mut self.queued_events,
            },
        })
    }

    /// Retrieves a mutable reference to the [`Asset`] with the given `id`, if it exists.
    ///
    /// This is the same as [`Assets::get_mut`] except it doesn't emit [`AssetEvent::Modified`].
    #[inline]
    pub fn get_mut_untracked(&mut self, id: impl Into<AssetId<A>>) -> Option<&mut A> {
        let id: AssetId<A> = id.into();
        match id {
            AssetId::Index { index, .. } => self.dense_storage.get_mut(index),
            AssetId::Uuid { uuid } => self.hash_map.get_mut(&uuid),
        }
    }

    /// Removes (and returns) the [`Asset`] with the given `id`, if it exists.
    /// Note that this supports anything that implements `Into<AssetId<A>>`, which includes [`Handle`] and [`AssetId`].
    pub fn remove(&mut self, id: impl Into<AssetId<A>>) -> Option<A> {
        let id: AssetId<A> = id.into();
        let result = self.remove_untracked(id);
        if result.is_some() {
            self.queued_events.push(AssetEvent::Removed { id });
        }
        result
    }

    /// Removes (and returns) the [`Asset`] with the given `id`, if it exists. This skips emitting [`AssetEvent::Removed`].
    /// Note that this supports anything that implements `Into<AssetId<A>>`, which includes [`Handle`] and [`AssetId`].
    ///
    /// This is the same as [`Assets::remove`] except it doesn't emit [`AssetEvent::Removed`].
    pub fn remove_untracked(&mut self, id: impl Into<AssetId<A>>) -> Option<A> {
        let id: AssetId<A> = id.into();
        match id {
            AssetId::Index { index, .. } => {
                self.duplicate_handles.remove(&index);
                self.dense_storage.remove_still_alive(index)
            }
            AssetId::Uuid { uuid } => self.hash_map.remove(&uuid),
        }
    }

    /// Removes the [`Asset`] with the given `id`.
    pub(crate) fn remove_dropped(&mut self, index: AssetIndex) {
        match self.duplicate_handles.get_mut(&index) {
            None => {}
            Some(0) => {
                self.duplicate_handles.remove(&index);
            }
            Some(value) => {
                *value -= 1;
                return;
            }
        }

        let existed = self.dense_storage.remove_dropped(index).is_some();

        self.queued_events
            .push(AssetEvent::Unused { id: index.into() });
        if existed {
            self.queued_events
                .push(AssetEvent::Removed { id: index.into() });
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
    /// [`Handle`] drop events.
    pub fn track_assets(mut assets: ResMut<Self>, asset_server: Res<AssetServer>) {
        let assets = &mut *assets;
        // note that we must hold this lock for the entire duration of this function to ensure
        // that `asset_server.load` calls that occur during it block, which ensures that
        // re-loads are kicked off appropriately. This function must be "transactional" relative
        // to other asset info operations
        let mut infos = asset_server.write_infos();
        while let Ok(drop_event) = assets.handle_provider.drop_receiver.try_recv() {
            if drop_event.asset_server_managed {
                // the process_handle_drop call checks whether new handles have been created since the drop event was fired, before removing the asset
                if !infos.process_handle_drop(drop_event.index) {
                    // a new handle has been created, or the asset doesn't exist
                    continue;
                }
            }

            assets.remove_dropped(drop_event.index.index);
        }
    }

    /// A system that applies accumulated asset change events to the [`Messages`] resource.
    ///
    /// [`Messages`]: bevy_ecs::message::Messages
    pub(crate) fn asset_events(
        mut assets: ResMut<Self>,
        mut messages: MessageWriter<AssetEvent<A>>,
        asset_changes: Option<ResMut<AssetChanges<A>>>,
        ticks: SystemChangeTick,
    ) {
        use AssetEvent::{Added, LoadedWithDependencies, Modified, Removed};

        if let Some(mut asset_changes) = asset_changes {
            for new_event in &assets.queued_events {
                match new_event {
                    Removed { id } | AssetEvent::Unused { id } => asset_changes.remove(id),
                    Added { id } | Modified { id } | LoadedWithDependencies { id } => {
                        asset_changes.insert(*id, ticks.this_run());
                    }
                };
            }
        }
        messages.write_batch(assets.queued_events.drain(..));
    }

    /// A run condition for [`asset_events`]. The system will not run if there are no events to
    /// flush.
    ///
    /// [`asset_events`]: Self::asset_events
    pub(crate) fn asset_events_condition(assets: Res<Self>) -> bool {
        !assets.queued_events.is_empty()
    }
}

/// Unique mutable borrow of an asset.
///
/// [`AssetEvent::Modified`] events will be only triggered if an asset itself is mutably borrowed.
///
/// Just as an example, this allows checking if a material property has changed
/// before modifying it to avoid unnecessary material extraction down the pipeline.
pub struct AssetMut<'a, A: Asset> {
    asset: &'a mut A,
    guard: AssetMutChangeNotifier<'a, A>,
}

impl<'a, A: Asset> AssetMut<'a, A> {
    /// Marks with inner asset as modified and returns reference to it.
    pub fn into_inner(mut self) -> &'a mut A {
        self.guard.changed = true;
        self.asset
    }

    /// Returns reference to the inner asset but doesn't mark it as modified.
    pub fn into_inner_untracked(self) -> &'a mut A {
        self.asset
    }
}

impl<'a, A: Asset> Deref for AssetMut<'a, A> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        self.asset
    }
}

impl<'a, A: Asset> DerefMut for AssetMut<'a, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.changed = true;
        self.asset
    }
}

/// Helper struct to allow safe destructuring of the [`AssetMut::into_inner`]
/// while also keeping strong change tracking guarantees.
struct AssetMutChangeNotifier<'a, A: Asset> {
    changed: bool,
    asset_id: AssetId<A>,
    queued_events: &'a mut Vec<AssetEvent<A>>,
}

impl<'a, A: Asset> Drop for AssetMutChangeNotifier<'a, A> {
    fn drop(&mut self) {
        if self.changed {
            self.queued_events
                .push(AssetEvent::Modified { id: self.asset_id });
        }
    }
}

/// A mutable iterator over [`Assets`].
pub struct AssetsMutIterator<'a, A: Asset> {
    queued_events: &'a mut Vec<AssetEvent<A>>,
    dense_storage: Enumerate<core::slice::IterMut<'a, Entry<A>>>,
    hash_map: bevy_platform::collections::hash_map::IterMut<'a, Uuid, A>,
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

/// An error returned when an [`AssetIndex`] has an invalid generation.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum InvalidGenerationError {
    #[error("AssetIndex {index:?} has an invalid generation. The current generation is: '{current_generation}'.")]
    Occupied {
        index: AssetIndex,
        current_generation: u32,
    },
    #[error("AssetIndex {index:?} has been removed")]
    Removed { index: AssetIndex },
}

#[cfg(test)]
mod test {
    use crate::{Asset, AssetApp, AssetEvent, AssetIndex, AssetPlugin, Assets};
    use bevy_app::{App, Last, Update};
    use bevy_ecs::prelude::*;
    use bevy_reflect::TypePath;

    #[test]
    fn asset_index_round_trip() {
        let asset_index = AssetIndex {
            generation: 42,
            index: 1337,
        };
        let roundtripped = AssetIndex::from_bits(asset_index.to_bits());
        assert_eq!(asset_index, roundtripped);
    }

    #[test]
    fn assets_mut_change_detection() {
        #[derive(Asset, TypePath, Default)]
        struct TestAsset {
            value: u32,
        }

        #[derive(Resource, Default)]
        struct TestState {
            asset_target_value: u32,
            asset_modified_counter: u32,
        }

        let mut app = App::new();
        app.add_plugins(AssetPlugin::default());
        app.init_asset::<TestAsset>();
        app.insert_resource(TestState::default());

        let mut assets = app.world_mut().resource_mut::<Assets<TestAsset>>();
        let my_asset_handle = assets.add(TestAsset::default());
        let my_asset_id = my_asset_handle.id();

        app.add_systems(
            Update,
            move |mut assets: ResMut<Assets<TestAsset>>, state: Res<TestState>| {
                let mut asset = assets.get_mut(my_asset_id).unwrap();

                if asset.value != state.asset_target_value {
                    asset.value = state.asset_target_value;
                }
            },
        );
        app.add_systems(
            Last,
            move |mut reader: MessageReader<AssetEvent<TestAsset>>,
                  mut state: ResMut<TestState>| {
                for event in reader.read() {
                    if event.is_modified(my_asset_id) {
                        state.asset_modified_counter += 1;
                    }
                }
            },
        );

        // check a few times just in case there are some unexpected leftover events from previous runs
        for _ in 0..3 {
            let mut state = app.world_mut().resource_mut::<TestState>();
            state.asset_target_value += 1;
            state.asset_modified_counter = 0;

            app.update();

            let mut state = app.world_mut().resource_mut::<TestState>();
            assert_eq!(
                core::mem::take(&mut state.asset_modified_counter),
                1,
                "Asset value was changed but AssetEvent::Modified was not triggered",
            );

            app.update();

            let mut state = app.world_mut().resource_mut::<TestState>();
            assert_eq!(
                core::mem::take(&mut state.asset_modified_counter),
                0,
                "Asset value was not changed but AssetEvent::Modified was triggered",
            );
        }
    }
}
