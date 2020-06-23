use crate::borrow::{AtomicRefCell, Ref, RefMap, RefMapMut, RefMut};
use crate::entity::Entity;
use crate::entity::EntityLocation;
use crate::event::EventFilterWrapper;
use crate::event::Subscriber;
use crate::event::{Event, Subscribers};
use crate::filter::ArchetypeFilterData;
use crate::filter::ChunkFilterData;
use crate::filter::ChunksetFilterData;
use crate::filter::EntityFilter;
use crate::filter::Filter;
use crate::index::ArchetypeIndex;
use crate::index::ChunkIndex;
use crate::index::ComponentIndex;
use crate::index::SetIndex;
use crate::iterator::FissileZip;
use crate::iterator::SliceVecIter;
use crate::world::TagSet;
use crate::world::WorldId;
use derivative::Derivative;
use fxhash::FxHashMap;
use smallvec::SmallVec;
use std::any::type_name;
use std::cell::UnsafeCell;
use std::fmt::Debug;
use std::fmt::{Display, Formatter};
use std::mem::size_of;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ops::RangeBounds;
use std::ptr::NonNull;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tracing::trace;

static VERSION_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_version() -> u64 {
    VERSION_COUNTER
        .fetch_add(1, Ordering::Relaxed)
        .checked_add(1)
        .unwrap()
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct ComponentTypeId(pub &'static str);

impl ComponentTypeId {
    /// Gets the component type ID that represents type `T`.
    pub fn of<T: Component>() -> Self { Self(type_name::<T>()) }
}

impl Display for ComponentTypeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
}

/// A type ID identifying a tag type.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct TagTypeId(pub &'static str);

impl TagTypeId {
    /// Gets the tag type ID that represents type `T`.
    pub fn of<T: Component>() -> Self { Self(type_name::<T>()) }
}

/// A `Component` is per-entity data that can be attached to a single entity.
pub trait Component: Send + Sync + 'static {}

/// A `Tag` is shared data that can be attached to multiple entities at once.
pub trait Tag: Clone + Send + Sync + PartialEq + 'static {}

impl<T: Send + Sync + 'static> Component for T {}
impl<T: Clone + Send + Sync + PartialEq + 'static> Tag for T {}

/// Stores slices of `ComponentTypeId`, each of which identifies the type of components
/// contained within the archetype of the same index.
#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct ComponentTypes(SliceVec<ComponentTypeId>);

/// Stores slices of `TagTypeId`, each of which identifies the type of tags
/// contained within the archetype of the same index.
#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct TagTypes(SliceVec<TagTypeId>);

impl ComponentTypes {
    /// Gets an iterator over all type ID slices.
    pub fn iter(&self) -> SliceVecIter<ComponentTypeId> { self.0.iter() }

    /// Gets the number of slices stored within the set.
    pub fn len(&self) -> usize { self.0.len() }

    /// Determines if the set is empty.
    pub fn is_empty(&self) -> bool { self.len() < 1 }
}

impl TagTypes {
    /// Gets an iterator over all type ID slices.
    pub fn iter(&self) -> SliceVecIter<TagTypeId> { self.0.iter() }

    /// Gets the number of slices stored within the set.
    pub fn len(&self) -> usize { self.0.len() }

    /// Determines if the set is empty.
    pub fn is_empty(&self) -> bool { self.len() < 1 }
}

/// A vector of slices.
///
/// Each slice is stored inline so as to be efficiently iterated through linearly.
#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct SliceVec<T> {
    data: Vec<T>,
    counts: Vec<usize>,
}

impl<T> SliceVec<T> {
    /// Gets the length of the vector.
    pub fn len(&self) -> usize { self.counts.len() }

    /// Determines if the vector is empty.
    pub fn is_empty(&self) -> bool { self.len() < 1 }

    /// Pushes a new slice onto the end of the vector.
    pub fn push<I: IntoIterator<Item = T>>(&mut self, items: I) {
        let mut count = 0;
        for item in items.into_iter() {
            self.data.push(item);
            count += 1;
        }
        self.counts.push(count);
    }

    /// Gets an iterator over all slices in the vector.
    pub fn iter(&self) -> SliceVecIter<T> {
        SliceVecIter {
            data: &self.data,
            counts: &self.counts,
        }
    }
}

/// Stores all entity data for a `World`.
pub struct Storage {
    world_id: WorldId,
    component_types: ComponentTypes,
    tag_types: TagTypes,
    archetypes: Vec<ArchetypeData>,
    subscribers: Subscribers,
}

impl Storage {
    // Creates an empty `Storage`.
    pub fn new(world_id: WorldId) -> Self {
        Self {
            world_id,
            component_types: ComponentTypes::default(),
            tag_types: TagTypes::default(),
            archetypes: Vec::default(),
            subscribers: Subscribers::default(),
        }
    }

    pub(crate) fn subscribe<T: EntityFilter + Sync + 'static>(
        &mut self,
        sender: crossbeam_channel::Sender<Event>,
        filter: T,
    ) {
        let subscriber = Subscriber::new(Arc::new(EventFilterWrapper(filter.clone())), sender);
        self.subscribers.push(subscriber.clone());

        for i in filter.iter_archetype_indexes(self).collect::<Vec<_>>() {
            self.archetypes_mut()[i].subscribe(subscriber.clone());
        }
    }

    /// Creates a new archetype.
    ///
    /// Returns the index of the newly created archetype and an exclusive reference to the
    /// achetype's data.
    pub(crate) fn alloc_archetype(
        &mut self,
        desc: ArchetypeDescription,
    ) -> (ArchetypeIndex, &mut ArchetypeData) {
        let index = ArchetypeIndex(self.archetypes.len());
        let id = ArchetypeId(self.world_id, index);
        let archetype = ArchetypeData::new(id, desc);

        self.push(archetype);

        let archetype = &mut self.archetypes[index];
        (index, archetype)
    }

    pub(crate) fn push(&mut self, mut archetype: ArchetypeData) {
        let desc = archetype.description();
        self.component_types
            .0
            .push(desc.components.iter().map(|&(t, _)| t));
        self.tag_types.0.push(desc.tags.iter().map(|&(t, _)| t));

        let index = ArchetypeIndex(self.archetypes.len());
        let archetype_data = ArchetypeFilterData {
            component_types: &self.component_types,
            tag_types: &self.tag_types,
        };

        let id = archetype.id();

        trace!(
            world = id.world().index(),
            archetype = *id.index(),
            components = ?desc.component_names,
            tags = ?desc.tag_names,
            "Created Archetype"
        );

        let mut subscribers = self.subscribers.matches_archetype(archetype_data, index);
        subscribers.send(Event::ArchetypeCreated(id));
        archetype.set_subscribers(subscribers);

        self.archetypes.push(archetype);
    }

    /// Gets a vector of slices of all component types for all archetypes.
    ///
    /// Each slice contains the component types for the archetype at the corresponding index.
    pub fn component_types(&self) -> &ComponentTypes { &self.component_types }

    /// Gets a vector of slices of all tag types for all archetypes.
    ///
    /// Each slice contains the tag types for the archetype at the corresponding index.
    pub fn tag_types(&self) -> &TagTypes { &self.tag_types }

    /// Gets a slice reference to all archetypes.
    pub fn archetypes(&self) -> &[ArchetypeData] { &self.archetypes }

    /// Gets a mutable slice reference to all archetypes.
    pub fn archetypes_mut(&mut self) -> &mut [ArchetypeData] { &mut self.archetypes }

    pub(crate) fn drain<R: RangeBounds<usize>>(
        &mut self,
        range: R,
    ) -> std::vec::Drain<ArchetypeData> {
        self.archetypes.drain(range)
    }

    pub(crate) fn archetype(
        &self,
        ArchetypeIndex(index): ArchetypeIndex,
    ) -> Option<&ArchetypeData> {
        self.archetypes().get(index)
    }

    pub(crate) fn archetype_mut(
        &mut self,
        ArchetypeIndex(index): ArchetypeIndex,
    ) -> Option<&mut ArchetypeData> {
        self.archetypes_mut().get_mut(index)
    }

    pub(crate) unsafe fn archetype_unchecked(
        &self,
        ArchetypeIndex(index): ArchetypeIndex,
    ) -> &ArchetypeData {
        self.archetypes().get_unchecked(index)
    }

    pub(crate) unsafe fn archetype_unchecked_mut(
        &mut self,
        ArchetypeIndex(index): ArchetypeIndex,
    ) -> &mut ArchetypeData {
        self.archetypes_mut().get_unchecked_mut(index)
    }

    pub(crate) fn chunk(&self, loc: EntityLocation) -> Option<&ComponentStorage> {
        self.archetype(loc.archetype())
            .and_then(|atd| atd.chunkset(loc.set()))
            .and_then(|cs| cs.chunk(loc.chunk()))
    }

    pub(crate) fn chunk_mut(&mut self, loc: EntityLocation) -> Option<&mut ComponentStorage> {
        self.archetype_mut(loc.archetype())
            .and_then(|atd| atd.chunkset_mut(loc.set()))
            .and_then(|cs| cs.chunk_mut(loc.chunk()))
    }
}

/// Stores metadata decribing the type of a tag.
#[derive(Copy, Clone, PartialEq)]
pub struct TagMeta {
    size: usize,
    align: usize,
    drop_fn: Option<fn(*mut u8)>,
    eq_fn: fn(*const u8, *const u8) -> bool,
    clone_fn: fn(*const u8, *mut u8),
}

impl TagMeta {
    /// Gets the tag meta of tag type `T`.
    pub fn of<T: Tag>() -> Self {
        TagMeta {
            size: size_of::<T>(),
            align: std::mem::align_of::<T>(),
            drop_fn: if std::mem::needs_drop::<T>() {
                Some(|ptr| unsafe { std::ptr::drop_in_place(ptr as *mut T) })
            } else {
                None
            },
            eq_fn: |a, b| unsafe { *(a as *const T) == *(b as *const T) },
            clone_fn: |src, dst| unsafe {
                let clone = (&*(src as *const T)).clone();
                std::ptr::write(dst as *mut T, clone);
            },
        }
    }

    pub(crate) unsafe fn equals(&self, a: *const u8, b: *const u8) -> bool { (self.eq_fn)(a, b) }

    pub(crate) unsafe fn clone(&self, src: *const u8, dst: *mut u8) { (self.clone_fn)(src, dst) }

    pub(crate) unsafe fn drop(&self, val: *mut u8) {
        if let Some(drop_fn) = self.drop_fn {
            (drop_fn)(val);
        }
    }

    pub(crate) fn layout(&self) -> std::alloc::Layout {
        unsafe { std::alloc::Layout::from_size_align_unchecked(self.size, self.align) }
    }

    pub(crate) fn is_zero_sized(&self) -> bool { self.size == 0 }
}

/// Stores metadata describing the type of a component.
#[derive(Copy, Clone, PartialEq)]
pub struct ComponentMeta {
    size: usize,
    align: usize,
    drop_fn: Option<fn(*mut u8)>,
}

impl ComponentMeta {
    /// Gets the component meta of component type `T`.
    pub fn of<T: Component>() -> Self {
        ComponentMeta {
            size: size_of::<T>(),
            align: std::mem::align_of::<T>(),
            drop_fn: if std::mem::needs_drop::<T>() {
                Some(|ptr| unsafe { std::ptr::drop_in_place(ptr as *mut T) })
            } else {
                None
            },
        }
    }

    pub(crate) fn size(&self) -> usize { self.size }

    pub(crate) fn align(&self) -> usize { self.align }
}

/// Describes the layout of an archetype, including what components
/// and tags shall be attached to entities stored within an archetype.
#[derive(Default, Clone, PartialEq)]
pub struct ArchetypeDescription {
    tags: Vec<(TagTypeId, TagMeta)>,
    components: Vec<(ComponentTypeId, ComponentMeta)>,
    tag_names: Vec<&'static str>,
    component_names: Vec<&'static str>,
}

impl ArchetypeDescription {
    /// Gets a slice of the tags in the description.
    pub fn tags(&self) -> &[(TagTypeId, TagMeta)] { &self.tags }

    /// Gets a slice of the components in the description.
    pub fn components(&self) -> &[(ComponentTypeId, ComponentMeta)] { &self.components }

    /// Adds a tag to the description.
    pub fn register_tag_raw(&mut self, type_id: TagTypeId, type_meta: TagMeta) {
        self.tags.push((type_id, type_meta));
        self.tag_names.push("<unknown>");
    }

    /// Adds a tag to the description.
    pub fn register_tag<T: Tag>(&mut self) {
        self.tags.push((TagTypeId::of::<T>(), TagMeta::of::<T>()));
        self.tag_names.push(std::any::type_name::<T>());
    }

    /// Adds a component to the description.
    pub fn register_component_raw(&mut self, type_id: ComponentTypeId, type_meta: ComponentMeta) {
        self.components.push((type_id, type_meta));
        self.component_names.push("<unknown>");
    }

    /// Adds a component to the description.
    pub fn register_component<T: Component>(&mut self) {
        self.components
            .push((ComponentTypeId::of::<T>(), ComponentMeta::of::<T>()));
        self.component_names.push(std::any::type_name::<T>());
    }
}

impl<'a> Filter<ArchetypeFilterData<'a>> for ArchetypeDescription {
    type Iter = FissileZip<SliceVecIter<'a, TagTypeId>, SliceVecIter<'a, ComponentTypeId>>;

    fn collect(&self, source: ArchetypeFilterData<'a>) -> Self::Iter {
        FissileZip::new(source.tag_types.iter(), source.component_types.iter())
    }

    fn is_match(&self, (tags, components): &<Self::Iter as Iterator>::Item) -> Option<bool> {
        Some(
            tags.len() == self.tags.len()
                && self.tags.iter().all(|(t, _)| tags.contains(t))
                && components.len() == self.components.len()
                && self.components.iter().all(|(t, _)| components.contains(t)),
        )
    }
}

const MAX_CHUNK_SIZE: usize = 16 * 1024 * 10;
const COMPONENT_STORAGE_ALIGNMENT: usize = 64;

/// Unique ID of an archetype.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ArchetypeId(WorldId, ArchetypeIndex);

impl ArchetypeId {
    pub(crate) fn new(world_id: WorldId, index: ArchetypeIndex) -> Self {
        ArchetypeId(world_id, index)
    }

    pub fn index(self) -> ArchetypeIndex { self.1 }

    pub fn world(self) -> WorldId { self.0 }
}

/// Contains all of the tags attached to the entities in each chunk.
pub struct Tags(pub(crate) SmallVec<[(TagTypeId, TagStorage); 3]>);

impl Tags {
    fn new(mut data: SmallVec<[(TagTypeId, TagStorage); 3]>) -> Self {
        data.sort_by_key(|&(t, _)| t);
        Self(data)
    }

    fn validate(&self, set_count: usize) {
        for (_, tags) in self.0.iter() {
            debug_assert_eq!(set_count, tags.len());
        }
    }

    /// Gets the set of tag values of the specified type attached to all chunks.
    #[inline]
    pub fn get(&self, type_id: TagTypeId) -> Option<&TagStorage> {
        self.0
            .binary_search_by_key(&type_id, |&(t, _)| t)
            .ok()
            .map(|i| unsafe { &self.0.get_unchecked(i).1 })
    }

    /// Mutably gets the set of all tag values of the specified type attached to all chunks.
    #[inline]
    pub fn get_mut(&mut self, type_id: TagTypeId) -> Option<&mut TagStorage> {
        self.0
            .binary_search_by_key(&type_id, |&(t, _)| t)
            .ok()
            .map(move |i| unsafe { &mut self.0.get_unchecked_mut(i).1 })
    }

    pub(crate) fn tag_set(&self, SetIndex(index): SetIndex) -> DynamicTagSet {
        let mut tags = DynamicTagSet { tags: Vec::new() };

        unsafe {
            for &(type_id, ref storage) in self.0.iter() {
                let (ptr, element_size, count) = storage.data_raw();
                debug_assert!(index < count, "set index out of bounds");
                tags.push(
                    type_id,
                    *storage.element(),
                    NonNull::new(ptr.as_ptr().add(element_size * index)).unwrap(),
                );
            }
        }

        tags
    }
}

pub(crate) struct DynamicTagSet {
    // the pointer here is to heap allocated memory owned by the tag set
    tags: Vec<(TagTypeId, TagMeta, NonNull<u8>)>,
}

unsafe impl Send for DynamicTagSet {}

unsafe impl Sync for DynamicTagSet {}

impl DynamicTagSet {
    pub fn push(&mut self, type_id: TagTypeId, meta: TagMeta, value: NonNull<u8>) {
        // we clone the value here and take ownership of the copy
        unsafe {
            if meta.is_zero_sized() {
                self.tags
                    .push((type_id, meta, NonNull::new(meta.align as *mut u8).unwrap()));
            } else {
                let copy = std::alloc::alloc(meta.layout());
                meta.clone(value.as_ptr(), copy);
                self.tags.push((type_id, meta, NonNull::new(copy).unwrap()));
            }
        }
    }

    pub fn remove(&mut self, type_id: TagTypeId) {
        if let Some((i, _)) = self
            .tags
            .iter()
            .enumerate()
            .find(|(_, &(t, _, _))| t == type_id)
        {
            let (_, meta, ptr) = self.tags.remove(i);
            unsafe {
                // drop and dealloc the copy as we own this memory
                if let Some(drop_fn) = meta.drop_fn {
                    drop_fn(ptr.as_ptr());
                }

                if !meta.is_zero_sized() {
                    std::alloc::dealloc(ptr.as_ptr(), meta.layout());
                }
            }
        }
    }
}

impl TagSet for DynamicTagSet {
    fn write_tags(&self, tags: &mut Tags) {
        for &(type_id, ref meta, ptr) in self.tags.iter() {
            let storage = tags.get_mut(type_id).unwrap();
            unsafe {
                if meta.drop_fn.is_some() && !meta.is_zero_sized() {
                    // clone the value into temp storage then move it into the chunk
                    // we can dealloc the copy without dropping because the value
                    // is considered moved and will be dropped by the tag storage later
                    let copy = std::alloc::alloc(meta.layout());
                    meta.clone(ptr.as_ptr(), copy);
                    storage.push_raw(copy);
                    std::alloc::dealloc(copy, meta.layout());
                } else {
                    // copy the value directly into the tag storage
                    // if the value has no drop fn, then it is safe for us to make
                    // copies of the data without explicit clones
                    storage.push_raw(ptr.as_ptr())
                }
            }
        }
    }
}

impl Drop for DynamicTagSet {
    fn drop(&mut self) {
        // we own all of the vales in the set, so we need to drop and dealloc them
        for (_, meta, ptr) in self.tags.drain(..) {
            unsafe {
                let layout = std::alloc::Layout::from_size_align_unchecked(meta.size, meta.align);
                if let Some(drop_fn) = meta.drop_fn {
                    drop_fn(ptr.as_ptr());
                }
                if !meta.is_zero_sized() {
                    std::alloc::dealloc(ptr.as_ptr(), layout);
                }
            }
        }
    }
}

/// Stores entity data in chunks. All entities within an archetype have the same data layout
/// (component and tag types).
pub struct ArchetypeData {
    id: ArchetypeId,
    desc: ArchetypeDescription,
    tags: Tags,
    component_layout: ComponentStorageLayout,
    chunk_sets: Vec<Chunkset>,
    subscribers: Subscribers,
}

impl ArchetypeData {
    fn new(id: ArchetypeId, desc: ArchetypeDescription) -> Self {
        // create tag storage
        let tags = desc
            .tags
            .iter()
            .map(|&(type_id, meta)| (type_id, TagStorage::new(meta)))
            .collect();

        // create component data layout
        let max_component_size = desc
            .components
            .iter()
            .map(|(_, meta)| meta.size)
            .max()
            .unwrap_or(0);
        let entity_capacity = std::cmp::max(
            1,
            MAX_CHUNK_SIZE / std::cmp::max(max_component_size, size_of::<Entity>()),
        );
        let mut data_capacity = 0usize;
        let mut component_data_offsets = Vec::new();
        for &(type_id, meta) in desc.components.iter() {
            data_capacity = align_up(
                align_up(data_capacity, COMPONENT_STORAGE_ALIGNMENT),
                meta.align,
            );
            component_data_offsets.push((type_id, data_capacity, meta));
            data_capacity += meta.size * entity_capacity;
        }
        let data_alignment =
            std::alloc::Layout::from_size_align(data_capacity, COMPONENT_STORAGE_ALIGNMENT)
                .expect("invalid component data size/alignment");

        ArchetypeData {
            desc,
            id,
            tags: Tags::new(tags),
            component_layout: ComponentStorageLayout {
                capacity: entity_capacity,
                alloc_layout: data_alignment,
                data_layout: component_data_offsets,
            },
            chunk_sets: Vec::new(),
            subscribers: Subscribers::default(),
        }
    }

    pub(crate) fn delete_all(&mut self) {
        for set in &mut self.chunk_sets {
            // Clearing the chunk will Drop all the data
            set.chunks.clear();
        }
    }

    pub(crate) fn subscribe(&mut self, subscriber: Subscriber) {
        self.subscribers.push(subscriber.clone());

        for i in (0..self.chunk_sets.len()).map(SetIndex) {
            let filter = ChunksetFilterData {
                archetype_data: self,
            };

            if subscriber.filter.matches_chunkset(filter, i) {
                self.chunk_sets[i].subscribe(subscriber.clone());
            }
        }
    }

    pub(crate) fn set_subscribers(&mut self, subscribers: Subscribers) {
        self.subscribers = subscribers;

        for i in (0..self.chunk_sets.len()).map(SetIndex) {
            let filter = ChunksetFilterData {
                archetype_data: self,
            };

            let subscribers = self.subscribers.matches_chunkset(filter, i);
            self.chunk_sets[i].set_subscribers(subscribers);
        }
    }

    /// Gets the unique ID of this archetype.
    pub fn id(&self) -> ArchetypeId { self.id }

    fn find_chunk_set_by_tags(
        &self,
        other_tags: &Tags,
        other_set_index: SetIndex,
    ) -> Option<SetIndex> {
        // search for a matching chunk set
        let mut set_match = None;
        for self_set_index in 0..self.chunk_sets.len() {
            let self_set_index = SetIndex(self_set_index);
            let mut matches = true;
            for &(type_id, ref tags) in self.tags.0.iter() {
                unsafe {
                    let (self_tag_ptr, size, _) = tags.data_raw();
                    let (other_tag_ptr, _, _) = other_tags.get(type_id).unwrap().data_raw();

                    if !tags.element().equals(
                        self_tag_ptr.as_ptr().add(self_set_index.0 * size),
                        other_tag_ptr.as_ptr().add(other_set_index.0 * size),
                    ) {
                        matches = false;
                        break;
                    }
                }
            }

            if matches {
                set_match = Some(self_set_index);
                break;
            }
        }

        set_match
    }

    pub(crate) fn find_or_create_chunk_set_by_tags(
        &mut self,
        src_tags: &Tags,
        src_chunk_set_index: SetIndex,
    ) -> SetIndex {
        let dst_chunk_set_index = self.find_chunk_set_by_tags(src_tags, src_chunk_set_index);
        dst_chunk_set_index.unwrap_or_else(|| {
            self.alloc_chunk_set(|self_tags| {
                for (type_id, other_tags) in src_tags.0.iter() {
                    unsafe {
                        let (src, _, _) = other_tags.data_raw();
                        let dst = self_tags.get_mut(*type_id).unwrap().alloc_ptr();
                        other_tags.element().clone(src.as_ptr(), dst);
                    }
                }
            })
        })
    }

    pub(crate) fn move_from(&mut self, mut other: ArchetypeData) {
        let other_tags = &other.tags;
        for (other_index, mut set) in other.chunk_sets.drain(..).enumerate() {
            let other_index = SetIndex(other_index);
            let set_match = self.find_chunk_set_by_tags(&other_tags, other_index);

            if let Some(chunk_set) = set_match {
                // if we found a match, move the chunks into the set
                let target = &mut self.chunk_sets[chunk_set];
                for mut chunk in set.drain(..) {
                    chunk.mark_modified();
                    target.push(chunk);
                }
            } else {
                // if we did not find a match, clone the tags and move the set
                set.mark_modified();
                self.push(set, |self_tags| {
                    for &(type_id, ref other_tags) in other_tags.0.iter() {
                        unsafe {
                            let (src, _, _) = other_tags.data_raw();
                            let dst = self_tags.get_mut(type_id).unwrap().alloc_ptr();
                            other_tags.element().clone(src.as_ptr(), dst);
                        }
                    }
                });
            }
        }

        self.tags.validate(self.chunk_sets.len());
    }

    /// Given a source world and archetype, step through all of its chunks and copy the data in it
    /// into this archetype. The archetype index is provided so that we can produce EntityLocations
    /// During this process, we can replace pre-existing entities. This function assumes that any
    /// entity referenced in replace_mappings actually exists in the world. The public API in world
    /// checks this assumption and panics if it is violated.
    ///
    /// See also `clone_from_single`, which copies a specific entity
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn clone_from<
        's,
        CloneImplT: crate::world::CloneImpl,
        CloneImplResultT: crate::world::CloneImplResult,
        EntityReplacePolicyT: crate::world::EntityReplacePolicy<'s>,
    >(
        &mut self,
        src_world: &crate::world::World,
        src_archetype: &ArchetypeData,
        dst_archetype_index: ArchetypeIndex,
        dst_entity_allocator: &crate::guid_entity_allocator::GuidEntityAllocator,
        dst_entity_locations: &mut crate::entity::Locations,
        clone_impl: &CloneImplT,
        clone_impl_result: &mut CloneImplResultT,
        entity_replace_policy: &EntityReplacePolicyT,
    ) {
        // Iterate all the chunk sets within the source archetype
        let src_tags = &src_archetype.tags;
        for (src_chunk_set_index, src_chunk_set) in src_archetype.chunk_sets.iter().enumerate() {
            let src_chunk_set_index = SetIndex(src_chunk_set_index);
            let dst_chunk_set_index =
                self.find_or_create_chunk_set_by_tags(src_tags, src_chunk_set_index);

            // Iterate all the chunks within the source chunk set
            for (_src_chunk_idx, src_chunk) in src_chunk_set.chunks.iter().enumerate() {
                // Copy the data from source to destination. Continuously find or create chunks as
                // needed until we've copied all the data
                let mut entities_remaining = src_chunk.len();
                while entities_remaining > 0 {
                    // Get or allocate a chunk.. since we could be transforming to a larger component size, it's possible
                    // that even a brand-new, empty chunk won't be large enough to hold everything in the chunk we are copying from
                    let dst_free_chunk_index =
                        self.get_free_chunk(dst_chunk_set_index, entities_remaining);
                    let dst_chunk_set = &mut self.chunk_sets[dst_chunk_set_index];
                    let dst_chunk = &mut dst_chunk_set.chunks[dst_free_chunk_index];

                    // Determine how many entities we will write
                    let entities_to_write =
                        std::cmp::min(entities_remaining, dst_chunk.capacity() - dst_chunk.len());

                    // Prepare to write to the chunk storage
                    let mut writer = dst_chunk.writer();
                    let (dst_entities, dst_components) = writer.get();

                    // Find the region of memory we will be reading from in the source chunk
                    let src_begin_idx = ComponentIndex(src_chunk.len() - entities_remaining);
                    let src_end_idx = ComponentIndex(src_begin_idx.0 + entities_to_write);

                    let dst_begin_idx = ComponentIndex(dst_entities.len());
                    let dst_end_idx = ComponentIndex(dst_entities.len() + entities_to_write);

                    // Copy all the entities to the destination chunk. The normal case is that we simply allocate
                    // new entities.
                    //
                    // We also allow end-user to specify a HashMap<Entity, Entity>. The key is an Entity from
                    // the source chunk and the value is an Entity from the destination chunk. Rather than appending
                    // data to the destination chunk, we will *replace* the data, according to the mapping. This
                    // is specifically intended for use with hot-reloading data. When some source data is changed,
                    // we can use the mapping to respawn entities as needed using the new data.

                    // We know how many entities will be appended to this list
                    dst_entities.reserve(dst_entities.len() + entities_to_write);

                    for src_entity in &src_chunk.entities[src_begin_idx.0..src_end_idx.0] {
                        // Determine if there is an entity we will be replacing
                        let dst_entity = entity_replace_policy.get_dst_entity(*src_entity);

                        // The location of the next entity
                        let location = EntityLocation::new(
                            dst_archetype_index,
                            dst_chunk_set_index,
                            dst_free_chunk_index,
                            ComponentIndex(dst_entities.len()),
                        );

                        // Determine the Entity to use for this element
                        let dst_entity = if let Some(dst_entity) = dst_entity {
                            // We are replacing data
                            // Verify that the entity is alive.. this checks the index and version of the entity
                            // The entity should be alive because World::clone_from verifies this
                            debug_assert!(dst_entity_allocator.is_alive(dst_entity));
                            dst_entity
                        } else {
                            // We are appending data, allocate a new entity
                            dst_entity_allocator.create_entity()
                        };

                        dst_entity_locations.set(dst_entity, location);
                        dst_entities.push(dst_entity);

                        clone_impl_result.add_result(*src_entity, dst_entity);
                    }

                    ArchetypeData::clone_components(
                        clone_impl,
                        src_world,
                        src_archetype,
                        src_chunk,
                        src_begin_idx..src_end_idx,
                        &dst_entities[dst_begin_idx.0..dst_end_idx.0],
                        dst_components,
                        entities_to_write,
                    );

                    entities_remaining -= entities_to_write;
                }
            }
        }
    }

    /// Given a source world, archetype, and entity, copy it into this archetype. The archetype
    /// index is provided so that we can produce EntityLocations.
    /// During this process, we can replace a pre-existing entity. This function assumes that if
    /// replace_mapping is not none, that the entity exists. The public API in world checks this
    /// assumption and panics if it is violated.
    ///
    /// See also `clone_from`, which copies all data
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn clone_from_single<C: crate::world::CloneImpl>(
        &mut self,
        src_world: &crate::world::World,
        src_archetype: &ArchetypeData,
        src_location: &EntityLocation,
        dst_archetype_index: ArchetypeIndex,
        dst_entity_allocator: &crate::guid_entity_allocator::GuidEntityAllocator,
        dst_entity_locations: &mut crate::entity::Locations,
        clone_impl: &C,
        replace_mapping: Option<Entity>,
    ) -> Entity {
        // We are reading from a specific chunk set within the source archetype
        let src_tags = &src_archetype.tags;
        let src_chunk_set_index = src_location.set();
        let src_chunk_set = &src_archetype.chunk_sets[src_chunk_set_index];

        // Find or create the chunk set that matches the source chunk set
        let dst_chunk_set_index =
            self.find_or_create_chunk_set_by_tags(src_tags, src_chunk_set_index);

        // Get the source chunk
        let src_chunk_idx = src_location.chunk();
        let src_chunk = &src_chunk_set.chunks[src_chunk_idx];

        // Get or allocate a chunk.. since we could be transforming to a larger component size, it's possible
        // that even a brand-new, empty chunk won't be large enough to hold everything in the chunk we are copying from
        let dst_free_chunk_index = self.get_free_chunk(dst_chunk_set_index, 1);
        let dst_chunk_set = &mut self.chunk_sets[dst_chunk_set_index];
        let dst_chunk = &mut dst_chunk_set.chunks[dst_free_chunk_index];

        // Determine how many entities we will write
        let entities_to_write = 1;

        // Prepare to write to the chunk storage
        let mut writer = dst_chunk.writer();
        let (dst_entities, dst_components) = writer.get();

        // Find the region of memory we will be reading from in the source chunk
        let src_begin_idx = src_location.component();
        let src_end_idx = ComponentIndex(src_begin_idx.0 + 1);

        // We know how many entities will be appended to this list
        let dst_begin_idx = ComponentIndex(dst_entities.len());
        let dst_end_idx = ComponentIndex(dst_entities.len() + entities_to_write);

        // Copy the entity to the destination chunk. The normal case is that we simply allocate
        // a new entity.
        //
        // We also allow end-user to specify a Option<Entity>. The src Entity from will *replace* the
        // data of the given Entity

        // The location of the next entity
        let location = EntityLocation::new(
            dst_archetype_index,
            dst_chunk_set_index,
            dst_free_chunk_index,
            ComponentIndex(dst_entities.len()),
        );

        let dst_entity = if let Some(dst_entity) = replace_mapping {
            // We are replacing data
            // Verify that the entity is alive.. this checks the index and version of the entity
            // The entity should be alive because World::clone_from verifies this
            debug_assert!(dst_entity_allocator.is_alive(dst_entity));
            dst_entity
        } else {
            // We are appending data, allocate a new entity
            dst_entity_allocator.create_entity()
        };

        dst_entity_locations.set(dst_entity, location);
        dst_entities.push(dst_entity);

        ArchetypeData::clone_components(
            clone_impl,
            src_world,
            src_archetype,
            src_chunk,
            src_begin_idx..src_end_idx,
            &dst_entities[dst_begin_idx.0..dst_end_idx.0],
            dst_components,
            entities_to_write,
        );

        dst_entity
    }

    /// Implements shared logic between `clone_from` and `clone_from_single`. For every component type,
    /// in the given archetype,
    #[allow(clippy::too_many_arguments)]
    fn clone_components<C: crate::world::CloneImpl>(
        clone_impl: &C,
        src_world: &crate::world::World,
        src_archetype: &ArchetypeData,
        src_chunk: &ComponentStorage,
        src_range: core::ops::Range<ComponentIndex>,
        dst_entities: &[Entity],
        dst_components: &UnsafeCell<Components>,
        entities_to_write: usize,
    ) {
        for (src_type, _) in src_archetype.description().components() {
            let dst_components = unsafe { &mut *dst_components.get() };

            // Look up what type we should transform the data into (can be the same type, meaning it should be cloned)
            let (dst_type, _) = clone_impl.map_component_type(*src_type);

            // Create a writer that will insert the data into the destination chunk
            let mut dst_component_writer = dst_components
                .get_mut(dst_type)
                .expect("ComponentResourceSet missing in clone_from")
                .writer();

            // Find the data in the source chunk
            let src_component_storage = src_chunk
                .components(*src_type)
                .expect("ComponentResourceSet missing in clone_from");

            // Now copy the data
            unsafe {
                let (src_component_chunk_data, src_element_size, _) =
                    src_component_storage.data_raw();

                // offset to the first entity we want to copy from the source chunk
                let src_data = src_component_chunk_data.add(src_element_size * src_range.start.0);

                // allocate the space we need in the destination chunk
                let dst_data = dst_component_writer.reserve_raw(entities_to_write).as_ptr();

                // Delegate the clone operation to the provided CloneImpl
                clone_impl.clone_components(
                    src_world,
                    src_chunk,
                    src_range.clone(),
                    *src_type,
                    &src_chunk.entities[src_range.start.0..src_range.end.0],
                    dst_entities,
                    src_data,
                    dst_data,
                    entities_to_write,
                );
            }
        }
    }

    /// Iterate all entities in existence by iterating across archetypes, chunk sets, and chunks
    pub(crate) fn iter_entities<'a>(&'a self) -> impl Iterator<Item = Entity> + 'a {
        self.chunk_sets.iter().flat_map(move |set| {
            set.chunks
                .iter()
                .flat_map(move |chunk| chunk.entities().iter().copied())
        })
    }

    pub(crate) fn iter_entity_locations<'a>(
        &'a self,
        archetype_index: ArchetypeIndex,
    ) -> impl Iterator<Item = (Entity, EntityLocation)> + 'a {
        self.chunk_sets
            .iter()
            .enumerate()
            .flat_map(move |(set_index, set)| {
                set.chunks
                    .iter()
                    .enumerate()
                    .flat_map(move |(chunk_index, chunk)| {
                        chunk
                            .entities()
                            .iter()
                            .enumerate()
                            .map(move |(entity_index, &entity)| {
                                (
                                    entity,
                                    EntityLocation::new(
                                        archetype_index,
                                        SetIndex(set_index),
                                        ChunkIndex(chunk_index),
                                        ComponentIndex(entity_index),
                                    ),
                                )
                            })
                    })
            })
    }

    fn push<F: FnMut(&mut Tags)>(&mut self, set: Chunkset, mut initialize: F) {
        initialize(&mut self.tags);
        self.chunk_sets.push(set);

        let index = SetIndex(self.chunk_sets.len() - 1);
        let filter = ChunksetFilterData {
            archetype_data: self,
        };
        let subscribers = self.subscribers.matches_chunkset(filter, index);

        self.chunk_sets[index].set_subscribers(subscribers);
        self.tags.validate(self.chunk_sets.len());
    }

    /// Allocates a new chunk set. Returns the index of the new set.
    ///
    /// `initialize` is expected to push the new chunkset's tag values onto the tags collection.
    pub(crate) fn alloc_chunk_set<F: FnMut(&mut Tags)>(&mut self, initialize: F) -> SetIndex {
        self.push(Chunkset::default(), initialize);
        SetIndex(self.chunk_sets.len() - 1)
    }

    /// Finds a chunk with space free for at least `minimum_space` entities, creating a chunk if needed.
    pub(crate) fn get_free_chunk(
        &mut self,
        set_index: SetIndex,
        minimum_space: usize,
    ) -> ChunkIndex {
        let count = {
            let chunks = &mut self.chunk_sets[set_index];
            let len = chunks.len();
            for (i, chunk) in chunks.iter_mut().enumerate() {
                let space_left = chunk.capacity() - chunk.len();
                if space_left >= minimum_space {
                    return ChunkIndex(i);
                }
            }
            ChunkIndex(len)
        };

        let chunk = self
            .component_layout
            .alloc_storage(ChunkId(self.id, set_index, count));
        unsafe { self.chunkset_unchecked_mut(set_index).push(chunk) };

        trace!(
            world = self.id.world().index(),
            archetype = *self.id.index(),
            chunkset = *set_index,
            chunk = *count,
            components = ?self.desc.component_names,
            tags = ?self.desc.tag_names,
            "Created chunk"
        );

        count
    }

    /// Gets the number of chunk sets stored within this archetype.
    pub fn len(&self) -> usize { self.chunk_sets.len() }

    /// Determines whether this archetype has any chunks.
    pub fn is_empty(&self) -> bool { self.len() < 1 }

    /// Gets the tag storage for all chunks in the archetype.
    pub fn tags(&self) -> &Tags { &self.tags }

    /// Mutably gets the tag storage for all chunks in the archetype.
    pub fn tags_mut(&mut self) -> &mut Tags { &mut self.tags }

    /// Gets a slice of chunksets.
    pub fn chunksets(&self) -> &[Chunkset] { &self.chunk_sets }

    /// Gets a mutable slice of chunksets.
    pub fn chunksets_mut(&mut self) -> &mut [Chunkset] { &mut self.chunk_sets }

    /// Gets a description of the component types in the archetype.
    pub fn description(&self) -> &ArchetypeDescription { &self.desc }

    pub(crate) fn defrag<F: FnMut(Entity, EntityLocation)>(
        &mut self,
        budget: &mut usize,
        mut on_moved: F,
    ) -> bool {
        trace!(
            world = self.id().world().index(),
            archetype = *self.id().index(),
            "Defragmenting archetype"
        );
        let arch_index = self.id.index();
        for (i, chunkset) in self.chunk_sets.iter_mut().enumerate() {
            let complete = chunkset.defrag(budget, |e, chunk, component| {
                on_moved(
                    e,
                    EntityLocation::new(arch_index, SetIndex(i), chunk, component),
                );
            });
            if !complete {
                return false;
            }
        }

        true
    }

    pub(crate) fn chunkset(&self, SetIndex(index): SetIndex) -> Option<&Chunkset> {
        self.chunksets().get(index)
    }

    pub(crate) fn chunkset_mut(&mut self, SetIndex(index): SetIndex) -> Option<&mut Chunkset> {
        self.chunksets_mut().get_mut(index)
    }

    pub(crate) unsafe fn chunkset_unchecked(&self, SetIndex(index): SetIndex) -> &Chunkset {
        self.chunksets().get_unchecked(index)
    }

    pub(crate) unsafe fn chunkset_unchecked_mut(
        &mut self,
        SetIndex(index): SetIndex,
    ) -> &mut Chunkset {
        self.chunksets_mut().get_unchecked_mut(index)
    }

    pub(crate) fn iter_data_slice<'a, T: Component>(
        &'a self,
    ) -> impl Iterator<Item = RefMap<&[T]>> + 'a {
        self.chunk_sets.iter().flat_map(move |set| {
            set.chunks.iter().map(move |chunk| {
                let c = chunk.components(ComponentTypeId::of::<T>()).unwrap();
                unsafe { c.data_slice::<T>() }
            })
        })
    }

    pub(crate) unsafe fn iter_data_slice_unchecked_mut<'a, T: Component>(
        &'a self,
    ) -> impl Iterator<Item = RefMapMut<&mut [T]>> + 'a {
        self.chunk_sets.iter().flat_map(move |set| {
            set.chunks.iter().map(move |chunk| {
                let c = chunk.components(ComponentTypeId::of::<T>()).unwrap();
                c.data_slice_mut::<T>()
            })
        })
    }
}

fn align_up(addr: usize, align: usize) -> usize { (addr + (align - 1)) & align.wrapping_neg() }

/// Describes the data layout for a chunk.
pub struct ComponentStorageLayout {
    capacity: usize,
    alloc_layout: std::alloc::Layout,
    data_layout: Vec<(ComponentTypeId, usize, ComponentMeta)>,
}

impl ComponentStorageLayout {
    /// The maximum number of entities that can be stored in each chunk.
    pub fn capacity(&self) -> usize { self.capacity }

    /// The components in each chunk.
    pub fn components(&self) -> &[(ComponentTypeId, usize, ComponentMeta)] { &self.data_layout }

    fn alloc_storage(&self, id: ChunkId) -> ComponentStorage {
        let storage_info = self
            .data_layout
            .iter()
            .map(|&(ty, _, ref meta)| {
                (
                    ty,
                    ComponentResourceSet {
                        ptr: AtomicRefCell::new(meta.align as *mut u8),
                        capacity: self.capacity,
                        count: UnsafeCell::new(0),
                        element_size: meta.size,
                        drop_fn: meta.drop_fn,
                        version: UnsafeCell::new(0),
                    },
                )
            })
            .collect();

        ComponentStorage {
            id,
            capacity: self.capacity,
            entities: Vec::with_capacity(self.capacity),
            component_offsets: self
                .data_layout
                .iter()
                .map(|&(ty, offset, _)| (ty, offset))
                .collect(),
            component_layout: self.alloc_layout,
            component_info: UnsafeCell::new(Components::new(storage_info)),
            component_data: None,
            subscribers: Subscribers::default(),
        }
    }
}

/// Contains chunks with the same layout and tag values.
#[derive(Default)]
pub struct Chunkset {
    chunks: Vec<ComponentStorage>,
    subscribers: Subscribers,
}

impl Deref for Chunkset {
    type Target = [ComponentStorage];

    fn deref(&self) -> &Self::Target { self.chunks.as_slice() }
}

impl DerefMut for Chunkset {
    fn deref_mut(&mut self) -> &mut Self::Target { self.chunks.as_mut_slice() }
}

impl Chunkset {
    pub(crate) fn new() -> Self {
        Self {
            chunks: Vec::new(),
            subscribers: Subscribers::default(),
        }
    }

    /// Pushes a new chunk into the set.
    pub fn push(&mut self, chunk: ComponentStorage) {
        let id = chunk.id();
        self.chunks.push(chunk);

        let index = ChunkIndex(self.chunks.len() - 1);
        let filter = ChunkFilterData {
            chunks: &self.chunks,
        };
        let mut subscribers = self.subscribers.matches_chunk(filter, index);
        subscribers.send(Event::ChunkCreated(id));
        self.chunks[index].set_subscribers(subscribers);
    }

    pub(crate) fn subscribe(&mut self, subscriber: Subscriber) {
        self.subscribers.push(subscriber.clone());

        for i in (0..self.chunks.len()).map(ChunkIndex) {
            let filter = ChunkFilterData {
                chunks: &self.chunks,
            };

            if subscriber.filter.matches_chunk(filter, i) {
                self.chunks[i].subscribe(subscriber.clone());
            }
        }
    }

    pub(crate) fn set_subscribers(&mut self, subscribers: Subscribers) {
        self.subscribers = subscribers;

        for i in (0..self.chunks.len()).map(ChunkIndex) {
            let filter = ChunkFilterData {
                chunks: &self.chunks,
            };

            let subscribers = self.subscribers.matches_chunk(filter, i);
            self.chunks[i].set_subscribers(subscribers);
        }
    }

    fn mark_modified(&mut self) {
        for chunk in self.chunks.iter_mut() {
            chunk.mark_modified();
        }
    }

    pub(crate) fn drain<R: RangeBounds<usize>>(
        &mut self,
        range: R,
    ) -> std::vec::Drain<ComponentStorage> {
        self.chunks.drain(range)
    }

    /// Gets a slice reference to occupied chunks.
    pub fn occupied(&self) -> &[ComponentStorage] {
        let mut len = self.chunks.len();
        while len > 0 {
            if unsafe { !self.chunks.get_unchecked(len - 1).is_empty() } {
                break;
            }
            len -= 1;
        }
        let (some, _) = self.chunks.as_slice().split_at(len);
        some
    }

    /// Gets a mutable slice reference to occupied chunks.
    pub fn occupied_mut(&mut self) -> &mut [ComponentStorage] {
        let mut len = self.chunks.len();
        while len > 0 {
            if unsafe { !self.chunks.get_unchecked(len - 1).is_empty() } {
                break;
            }
            len -= 1;
        }
        let (some, _) = self.chunks.as_mut_slice().split_at_mut(len);
        some
    }

    /// Defragments all chunks within the chunkset.
    ///
    /// This will compact entities down into lower index chunks, preferring to fill one
    /// chunk before moving on to the next.
    ///
    /// `budget` determines the maximum number of entities that can be moved, and is decremented
    /// as this function moves entities.
    ///
    /// `on_moved` is called when an entity is moved, with the entity's ID, new chunk index,
    /// new component index.
    ///
    /// Returns whether or not the chunkset has been fully defragmented.
    fn defrag<F: FnMut(Entity, ChunkIndex, ComponentIndex)>(
        &mut self,
        budget: &mut usize,
        mut on_moved: F,
    ) -> bool {
        let slice = self.occupied_mut();

        if slice.is_empty() {
            return true;
        }

        let mut first = 0;
        let mut last = slice.len() - 1;

        trace!("Defragmenting chunkset");

        loop {
            // find the first chunk that is not full
            while first < last && slice[first].is_full() {
                first += 1;
            }

            // find the last chunk that is not empty
            while last > first && slice[last].is_empty() {
                last -= 1;
            }

            // exit if the cursors meet; the chunkset is defragmented
            if first == last {
                return true;
            }

            // get mut references to both chunks
            let (with_first, with_last) = slice.split_at_mut(last);
            let target = &mut with_first[first];
            let source = &mut with_last[0];

            // move as many entities as we can from the last chunk into the first
            loop {
                if *budget == 0 {
                    return false;
                }

                *budget -= 1;

                // move the last entity
                let comp_index = ComponentIndex(source.len() - 1);
                let swapped = source.move_entity(target, comp_index);
                assert!(swapped.is_none());

                // notify move
                on_moved(
                    *target.entities.last().unwrap(),
                    ChunkIndex(first),
                    comp_index,
                );

                // exit if we cant move any more
                if target.is_full() || source.is_empty() {
                    break;
                }
            }
        }
    }

    pub(crate) fn chunk(&self, ChunkIndex(index): ChunkIndex) -> Option<&ComponentStorage> {
        self.chunks.get(index)
    }

    pub(crate) fn chunk_mut(
        &mut self,
        ChunkIndex(index): ChunkIndex,
    ) -> Option<&mut ComponentStorage> {
        self.chunks.get_mut(index)
    }

    pub(crate) unsafe fn chunk_unchecked(
        &self,
        ChunkIndex(index): ChunkIndex,
    ) -> &ComponentStorage {
        self.chunks.get_unchecked(index)
    }

    pub(crate) unsafe fn chunk_unchecked_mut(
        &mut self,
        ChunkIndex(index): ChunkIndex,
    ) -> &mut ComponentStorage {
        self.chunks.get_unchecked_mut(index)
    }
}

/// Unique ID of a chunk.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ChunkId(ArchetypeId, SetIndex, ChunkIndex);

impl ChunkId {
    pub(crate) fn new(archetype: ArchetypeId, set: SetIndex, index: ChunkIndex) -> Self {
        ChunkId(archetype, set, index)
    }

    pub fn archetype_id(&self) -> ArchetypeId { self.0 }

    pub(crate) fn set(&self) -> SetIndex { self.1 }

    pub(crate) fn index(&self) -> ChunkIndex { self.2 }
}

/// A set of component slices located on a chunk.
pub struct Components(SmallVec<[(ComponentTypeId, ComponentResourceSet); 5]>);

impl Components {
    pub(crate) fn new(mut data: SmallVec<[(ComponentTypeId, ComponentResourceSet); 5]>) -> Self {
        data.sort_by_key(|&(t, _)| t);
        Self(data)
    }

    /// Gets a component slice accessor for the specified component type.
    #[inline]
    pub fn get(&self, type_id: ComponentTypeId) -> Option<&ComponentResourceSet> {
        self.0
            .binary_search_by_key(&type_id, |&(t, _)| t)
            .ok()
            .map(|i| unsafe { &self.0.get_unchecked(i).1 })
    }

    /// Gets a mutable component slice accessor for the specified component type.
    #[inline]
    pub fn get_mut(&mut self, type_id: ComponentTypeId) -> Option<&mut ComponentResourceSet> {
        self.0
            .binary_search_by_key(&type_id, |&(t, _)| t)
            .ok()
            .map(move |i| unsafe { &mut self.0.get_unchecked_mut(i).1 })
    }

    fn iter(&mut self) -> impl Iterator<Item = &(ComponentTypeId, ComponentResourceSet)> + '_ {
        self.0.iter()
    }

    fn iter_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut (ComponentTypeId, ComponentResourceSet)> + '_ {
        self.0.iter_mut()
    }

    fn drain(&mut self) -> impl Iterator<Item = (ComponentTypeId, ComponentResourceSet)> + '_ {
        self.0.drain(..)
    }
}

/// Stores a chunk of entities and their component data of a specific data layout.
pub struct ComponentStorage {
    id: ChunkId,
    capacity: usize,
    entities: Vec<Entity>,
    component_layout: std::alloc::Layout,
    component_offsets: FxHashMap<ComponentTypeId, usize>,
    component_info: UnsafeCell<Components>,
    component_data: Option<NonNull<u8>>,
    subscribers: Subscribers,
}

pub struct StorageWriter<'a> {
    initial_count: usize,
    storage: &'a mut ComponentStorage,
}

impl<'a> StorageWriter<'a> {
    pub fn get(&mut self) -> (&mut Vec<Entity>, &UnsafeCell<Components>) {
        (&mut self.storage.entities, &self.storage.component_info)
    }
}

impl<'a> Drop for StorageWriter<'a> {
    fn drop(&mut self) {
        self.storage.update_count_gauge();
        for &entity in self.storage.entities.iter().skip(self.initial_count) {
            self.storage
                .subscribers
                .send(Event::EntityInserted(entity, self.storage.id()));
        }
    }
}

impl ComponentStorage {
    /// Gets the unique ID of the chunk.
    pub fn id(&self) -> ChunkId { self.id }

    /// Gets the number of entities stored in the chunk.
    pub fn len(&self) -> usize { self.entities.len() }

    /// Gets the maximum number of entities that can be stored in the chunk.
    pub fn capacity(&self) -> usize { self.capacity }

    /// Determines if the chunk is full.
    pub fn is_full(&self) -> bool { self.len() >= self.capacity }

    /// Determines if the chunk is empty.
    pub fn is_empty(&self) -> bool { self.entities.len() == 0 }

    /// Determines if the internal memory for this chunk has been allocated.
    pub fn is_allocated(&self) -> bool { self.component_data.is_some() }

    pub(crate) fn subscribe(&mut self, subscriber: Subscriber) {
        self.subscribers.push(subscriber);
    }

    pub(crate) fn set_subscribers(&mut self, subscribers: Subscribers) {
        self.subscribers = subscribers;
    }

    /// Gets a slice reference containing the IDs of all entities stored in the chunk.
    pub fn entities(&self) -> &[Entity] { self.entities.as_slice() }

    /// Gets a component accessor for the specified component type.
    pub fn components(&self, component_type: ComponentTypeId) -> Option<&ComponentResourceSet> {
        unsafe { &*self.component_info.get() }.get(component_type)
    }

    /// Increments all component versions, forcing the chunk to be seen as modified for all queries.
    fn mark_modified(&mut self) {
        unsafe {
            let components = &mut *self.component_info.get();
            for (_, component) in components.iter_mut() {
                // touch each slice mutably to increment its version
                let _ = component.data_raw_mut();
            }
        }
    }

    /// Removes an entity from the chunk by swapping it with the last entry.
    ///
    /// Returns the ID of the entity which was swapped into the removed entity's position.
    pub fn swap_remove(
        &mut self,
        ComponentIndex(index): ComponentIndex,
        drop: bool,
    ) -> Option<Entity> {
        let removed = self.entities.swap_remove(index);
        for (_, component) in unsafe { &mut *self.component_info.get() }.iter_mut() {
            component.writer().swap_remove(index, drop);
        }

        self.subscribers
            .send(Event::EntityRemoved(removed, self.id()));
        self.update_count_gauge();

        if self.entities.len() > index {
            Some(*self.entities.get(index).unwrap())
        } else {
            if self.is_empty() {
                self.free();
            }

            None
        }
    }

    /// Moves an entity from this chunk into a target chunk, moving all compatable components into
    /// the target chunk. Any components left over will be dropped.
    ///
    /// Returns the ID of the entity which was swapped into the removed entity's position.
    pub fn move_entity(
        &mut self,
        target: &mut ComponentStorage,
        index: ComponentIndex,
    ) -> Option<Entity> {
        debug_assert!(*index < self.len());
        debug_assert!(!target.is_full());
        if !target.is_allocated() {
            target.allocate();
        }

        trace!(index = *index, source = ?self.id, destination = ?target.id, "Moving entity");

        let entity = unsafe { *self.entities.get_unchecked(*index) };
        target.entities.push(entity);

        let self_components = unsafe { &mut *self.component_info.get() };
        let target_components = unsafe { &mut *target.component_info.get() };

        for (comp_type, accessor) in self_components.iter_mut() {
            if let Some(target_accessor) = target_components.get_mut(*comp_type) {
                // move the component into the target chunk
                unsafe {
                    let (ptr, element_size, _) = accessor.data_raw();
                    let component = ptr.add(element_size * *index);
                    target_accessor
                        .writer()
                        .push_raw(NonNull::new_unchecked(component), 1);
                }
            } else {
                // drop the component rather than move it
                unsafe { accessor.writer().drop_in_place(index) };
            }
        }

        // remove the entity from this chunk
        let removed = self.swap_remove(index, false);

        target
            .subscribers
            .send(Event::EntityInserted(entity, target.id()));
        target.update_count_gauge();

        removed
    }

    /// Gets mutable references to the internal data of the chunk.
    pub fn writer(&mut self) -> StorageWriter {
        if !self.is_allocated() {
            self.allocate();
        }
        StorageWriter {
            initial_count: self.entities.len(),
            storage: self,
        }
    }

    fn free(&mut self) {
        debug_assert!(self.is_allocated());
        debug_assert_eq!(0, self.len());

        self.entities.shrink_to_fit();

        trace!(
            world = self.id.archetype_id().world().index(),
            archetype = *self.id.archetype_id().index(),
            chunkset = *self.id.set(),
            chunk = *self.id.index(),
            layout = ?self.component_layout,
            "Freeing chunk memory"
        );

        // Safety Note:
        // accessors are left with pointers pointing to invalid memory (although aligned properly)
        // the slices returned from these accessors will be empty though, so no code
        // should ever dereference these pointers

        // free component memory
        unsafe {
            let ptr = self.component_data.take().unwrap();

            if self.component_layout.size() > 0 {
                std::alloc::dealloc(ptr.as_ptr(), self.component_layout);
            }
        }

        self.update_mem_gauge();
    }

    fn allocate(&mut self) {
        debug_assert!(!self.is_allocated());

        trace!(
            world = self.id.archetype_id().world().index(),
            archetype = *self.id.archetype_id().index(),
            chunkset = *self.id.set(),
            chunk = *self.id.index(),
            layout = ?self.component_layout,
            "Allocating chunk memory"
        );
        self.entities.reserve_exact(self.capacity);

        unsafe {
            // allocating backing store
            if self.component_layout.size() > 0 {
                let ptr = std::alloc::alloc(self.component_layout);
                self.component_data = Some(NonNull::new_unchecked(ptr));

                // update accessor pointers
                for (type_id, component) in (&mut *self.component_info.get()).iter_mut() {
                    let &offset = self.component_offsets.get(type_id).unwrap();
                    *component.ptr.get_mut() = ptr.add(offset);
                }
            } else {
                self.component_data =
                    Some(NonNull::new(self.component_layout.align() as *mut u8).unwrap());
            }
        }

        self.update_mem_gauge();
    }

    fn update_mem_gauge(&self) {
        #[cfg(feature = "metrics")]
        {
            use std::convert::TryInto;
            metrics::gauge!(
                "chunk_memory",
                if self.is_allocated() { self.component_layout.size().try_into().unwrap() } else { 0 },
                "world" => self.id.archetype_id().world().index().to_string(),
                "archetype" => self.id.archetype_id().index().to_string(),
                "chunkset" => self.id.set().to_string(),
                "chunk" => self.id.index().to_string()
            );
        }
    }

    fn update_count_gauge(&self) {
        #[cfg(feature = "metrics")]
        {
            use std::convert::TryInto;
            metrics::gauge!(
                "entity_count",
                self.len().try_into().unwrap(),
                "world" => self.id.archetype_id().world().index().to_string(),
                "archetype" => self.id.archetype_id().index().to_string(),
                "chunkset" => self.id.set().to_string(),
                "chunk" => self.id.index().to_string()
            );
        }
    }
}

unsafe impl Sync for ComponentStorage {}

unsafe impl Send for ComponentStorage {}

impl Drop for ComponentStorage {
    fn drop(&mut self) {
        if let Some(ptr) = self.component_data {
            // run the drop functions of all components
            for (_, info) in unsafe { &mut *self.component_info.get() }.drain() {
                if let Some(drop_fn) = info.drop_fn {
                    let ptr = info.ptr.get_mut();
                    for i in 0..self.len() {
                        unsafe {
                            drop_fn(ptr.add(info.element_size * i));
                        }
                    }
                }
            }

            for e in &self.entities {
                self.subscribers.send(Event::EntityRemoved(*e, self.id()));
            }

            self.update_count_gauge();

            // free the chunk's memory
            if self.component_layout.size() > 0 {
                unsafe {
                    std::alloc::dealloc(ptr.as_ptr(), self.component_layout);
                }
            }
        }
    }
}

/// Provides raw access to component data slices.
#[repr(align(64))]
pub struct ComponentResourceSet {
    ptr: AtomicRefCell<*mut u8>,
    element_size: usize,
    count: UnsafeCell<usize>,
    capacity: usize,
    drop_fn: Option<fn(*mut u8)>,
    version: UnsafeCell<u64>,
}

impl ComponentResourceSet {
    /// Gets the version of the component slice.
    pub fn version(&self) -> u64 { unsafe { *self.version.get() } }

    /// Gets a raw pointer to the start of the component slice.
    ///
    /// Returns a tuple containing `(pointer, element_size, count)`.
    ///
    /// # Safety
    ///
    /// Access to the component data within the slice is runtime borrow checked in debug builds.
    /// This call will panic if borrowing rules are broken in debug, and is undefined behavior in release.
    pub unsafe fn data_raw(&self) -> (Ref<*mut u8>, usize, usize) {
        (self.ptr.get(), self.element_size, *self.count.get())
    }

    /// Gets a raw pointer to the start of the component slice.
    ///
    /// Returns a tuple containing `(pointer, element_size, count)`.
    ///
    /// # Safety
    ///
    /// Access to the component data within the slice is runtime borrow checked in debug builds.
    /// This call will panic if borrowing rules are broken in debug, and is undefined behavior in release.
    ///
    /// # Panics
    ///
    /// Will panic when an internal u64 counter overflows.
    /// It will happen in 50000 years if you do 10000 mutations a millisecond.
    pub unsafe fn data_raw_mut(&self) -> (RefMut<*mut u8>, usize, usize) {
        // this version increment is not thread safe
        // - but the pointer `get_mut` ensures exclusive access at runtime
        let ptr = self.ptr.get_mut();
        *self.version.get() = next_version();
        (ptr, self.element_size, *self.count.get())
    }

    /// Gets a shared reference to the slice of components.
    ///
    /// # Safety
    ///
    /// Ensure that `T` is representative of the component data actually stored.
    ///
    /// Access to the component data within the slice is runtime borrow checked.
    /// This call will panic if borrowing rules are broken.
    pub unsafe fn data_slice<T>(&self) -> RefMap<&[T]> {
        let (ptr, _size, count) = self.data_raw();
        ptr.map_into(|&ptr| std::slice::from_raw_parts(ptr as *const _ as *const T, count))
    }

    /// Gets a mutable reference to the slice of components.
    ///
    /// # Safety
    ///
    /// Ensure that `T` is representative of the component data actually stored.
    ///
    /// Access to the component data within the slice is runtime borrow checked.
    /// This call will panic if borrowing rules are broken.
    ///
    /// # Panics
    ///
    /// Will panic when an internal u64 counter overflows.
    /// It will happen in 50000 years if you do 10000 mutations a millisecond.
    pub unsafe fn data_slice_mut<T>(&self) -> RefMapMut<&mut [T]> {
        let (ptr, _size, count) = self.data_raw_mut();
        ptr.map_into(|&mut ptr| std::slice::from_raw_parts_mut(ptr as *mut _ as *mut T, count))
    }

    /// Creates a writer for pushing components into or removing from the vec.
    pub fn writer(&mut self) -> ComponentWriter { ComponentWriter::new(self) }
}

impl Debug for ComponentResourceSet {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "ComponentResourceSet {{ ptr: {:?}, element_size: {}, count: {}, capacity: {}, version: {} }}",
            *self.ptr.get(),
            self.element_size,
            unsafe { *self.count.get() },
            self.capacity,
            self.version()
        )
    }
}

/// Provides methods adding or removing components from a component vec.
pub struct ComponentWriter<'a> {
    accessor: &'a ComponentResourceSet,
    ptr: RefMut<'a, *mut u8>,
}

impl<'a> ComponentWriter<'a> {
    fn new(accessor: &'a ComponentResourceSet) -> ComponentWriter<'a> {
        Self {
            accessor,
            ptr: accessor.ptr.get_mut(),
        }
    }

    /// Increases the length of the associated `ComponentResourceSet` by `count`
    /// and returns a pointer to the start of the memory that is reserved as a result.
    ///
    /// # Safety
    ///
    /// Ensure the memory returned by this function is properly initialized before calling
    /// any other storage function. Ensure that the data written into the returned pointer
    /// is representative of the component types stored in the associated ComponentResourceSet.
    ///
    /// # Panics
    ///
    /// Will panic when an internal u64 counter overflows.
    /// It will happen in 50000 years if you do 10000 mutations a millisecond.
    pub(crate) unsafe fn reserve_raw(&mut self, count: usize) -> NonNull<u8> {
        debug_assert!((*self.accessor.count.get() + count) <= self.accessor.capacity);
        let ptr = self
            .ptr
            .add(*self.accessor.count.get() * self.accessor.element_size);
        *self.accessor.count.get() += count;
        *self.accessor.version.get() = next_version();
        NonNull::new_unchecked(ptr)
    }

    /// Pushes new components onto the end of the vec.
    ///
    /// # Safety
    ///
    /// Ensure the components pointed to by `components` are representative
    /// of the component types stored in the vec.
    ///
    /// This function will _copy_ all elements into the chunk. If the source is not `Copy`,
    /// the caller must then `mem::forget` the source such that the destructor does not run
    /// on the original data.
    ///
    /// # Panics
    ///
    /// Will panic when an internal u64 counter overflows.
    /// It will happen in 50000 years if you do 10000 mutations a millisecond.
    pub unsafe fn push_raw(&mut self, components: NonNull<u8>, count: usize) {
        debug_assert!((*self.accessor.count.get() + count) <= self.accessor.capacity);
        std::ptr::copy_nonoverlapping(
            components.as_ptr(),
            self.ptr
                .add(*self.accessor.count.get() * self.accessor.element_size),
            count * self.accessor.element_size,
        );
        *self.accessor.count.get() += count;
        *self.accessor.version.get() = next_version();
    }

    /// Pushes new components onto the end of the vec.
    ///
    /// # Safety
    ///
    /// Ensure that the type `T` is representative of the component types stored in the vec.
    ///
    /// This function will _copy_ all elements of `T` into the chunk. If `T` is not `Copy`,
    /// the caller must then `mem::forget` the source such that the destructor does not run
    /// on the original data.
    pub unsafe fn push<T: Component>(&mut self, components: &[T]) {
        self.push_raw(
            NonNull::new_unchecked(components.as_ptr() as *mut u8),
            components.len(),
        );
    }

    /// Removes the component at the specified index by swapping it with the last component.
    pub fn swap_remove(&mut self, index: usize, drop: bool) {
        unsafe {
            let size = self.accessor.element_size;
            let to_remove = self.ptr.add(size * index);
            if drop {
                if let Some(drop_fn) = self.accessor.drop_fn {
                    drop_fn(to_remove);
                }
            }

            let count = *self.accessor.count.get();
            if index < count - 1 {
                let swap_target = self.ptr.add(size * (count - 1));
                std::ptr::copy_nonoverlapping(swap_target, to_remove, size);
            }

            *self.accessor.count.get() -= 1;
        }
    }

    /// Drops the component stored at `index` without moving any other data or
    /// altering the number of elements.
    ///
    /// # Safety
    ///
    /// Ensure that this function is only ever called once on a given index.
    pub unsafe fn drop_in_place(&mut self, ComponentIndex(index): ComponentIndex) {
        if let Some(drop_fn) = self.accessor.drop_fn {
            let size = self.accessor.element_size;
            let to_remove = self.ptr.add(size * index);
            drop_fn(to_remove);
        }
    }
}

/// A vector of tag values of a single type.
///
/// Each element in the vector represents the value of tag for
/// the chunk with the corresponding index.
pub struct TagStorage {
    ptr: NonNull<u8>,
    capacity: usize,
    len: usize,
    element: TagMeta,
}

impl TagStorage {
    pub(crate) fn new(element: TagMeta) -> Self {
        let capacity = if element.size == 0 { !0 } else { 4 };

        let ptr = unsafe {
            if element.size > 0 {
                let layout =
                    std::alloc::Layout::from_size_align(capacity * element.size, element.align)
                        .unwrap();
                NonNull::new_unchecked(std::alloc::alloc(layout))
            } else {
                NonNull::new_unchecked(element.align as *mut u8)
            }
        };

        TagStorage {
            ptr,
            capacity,
            len: 0,
            element,
        }
    }

    /// Gets the element metadata.
    pub fn element(&self) -> &TagMeta { &self.element }

    /// Gets the number of tags contained within the vector.
    pub fn len(&self) -> usize { self.len }

    /// Determines if the vector is empty.
    pub fn is_empty(&self) -> bool { self.len() < 1 }

    /// Allocates uninitialized memory for a new element.
    ///
    /// # Safety
    ///
    /// A valid element must be written into the returned address before the
    /// tag storage is next accessed.
    pub unsafe fn alloc_ptr(&mut self) -> *mut u8 {
        if self.len == self.capacity {
            self.grow();
        }

        let ptr = if self.element.size > 0 {
            self.ptr.as_ptr().add(self.len * self.element.size)
        } else {
            self.element.align as *mut u8
        };

        self.len += 1;
        ptr
    }

    /// Pushes a new tag onto the end of the vector.
    ///
    /// # Safety
    ///
    /// Ensure the tag pointed to by `ptr` is representative
    /// of the tag types stored in the vec.
    ///
    /// `ptr` must not point to a location already within the vector.
    ///
    /// The value at `ptr` is _copied_ into the tag vector. If the value
    /// is not `Copy`, then the caller must ensure that the original value
    /// is forgotten with `mem::forget` such that the finalizer is not called
    /// twice.
    pub unsafe fn push_raw(&mut self, ptr: *const u8) {
        if self.len == self.capacity {
            self.grow();
        }

        if self.element.size > 0 {
            let dst = self.ptr.as_ptr().add(self.len * self.element.size);
            std::ptr::copy_nonoverlapping(ptr, dst, self.element.size);
        }

        self.len += 1;
    }

    /// Pushes a new tag onto the end of the vector.
    ///
    /// # Safety
    ///
    /// Ensure that the type `T` is representative of the tag type stored in the vec.
    pub unsafe fn push<T: Tag>(&mut self, value: T) {
        debug_assert!(
            size_of::<T>() == self.element.size,
            "incompatible element data size"
        );
        self.push_raw(&value as *const T as *const u8);
        std::mem::forget(value);
    }

    /// Gets a raw pointer to the start of the tag slice.
    ///
    /// Returns a tuple containing `(pointer, element_size, count)`.
    ///
    /// # Safety
    /// This function returns a raw pointer with the size and length.
    /// Ensure that you do not access outside these bounds for this pointer.
    pub unsafe fn data_raw(&self) -> (NonNull<u8>, usize, usize) {
        (self.ptr, self.element.size, self.len)
    }

    /// Gets a shared reference to the slice of tags.
    ///
    /// # Safety
    ///
    /// Ensure that `T` is representative of the tag data actually stored.
    ///
    /// Access to the tag data within the slice is runtime borrow checked.
    /// This call will panic if borrowing rules are broken.
    pub unsafe fn data_slice<T>(&self) -> &[T] {
        debug_assert!(
            size_of::<T>() == self.element.size,
            "incompatible element data size"
        );
        std::slice::from_raw_parts(self.ptr.as_ptr() as *const T, self.len)
    }

    /// Drop the storage without dropping the tags contained in the storage
    pub(crate) fn forget_data(mut self) {
        // this is a bit of a hack, but it makes the Drop impl not drop the elements
        self.element.drop_fn = None;
    }

    fn grow(&mut self) {
        assert!(self.element.size != 0, "capacity overflow");
        unsafe {
            let (new_cap, ptr) = {
                let layout = std::alloc::Layout::from_size_align(
                    self.capacity * self.element.size,
                    self.element.align,
                )
                .unwrap();
                let new_cap = 2 * self.capacity;
                let ptr =
                    std::alloc::realloc(self.ptr.as_ptr(), layout, new_cap * self.element.size);

                (new_cap, ptr)
            };

            if ptr.is_null() {
                tracing::error!("out of memory");
                std::process::abort()
            }

            self.ptr = NonNull::new_unchecked(ptr);
            self.capacity = new_cap;
        }
    }
}

unsafe impl Sync for TagStorage {}

unsafe impl Send for TagStorage {}

impl Drop for TagStorage {
    fn drop(&mut self) {
        if self.element.size > 0 {
            let ptr = self.ptr.as_ptr();

            unsafe {
                if let Some(drop_fn) = self.element.drop_fn {
                    for i in 0..self.len {
                        drop_fn(ptr.add(i * self.element.size));
                    }
                }
                let layout = std::alloc::Layout::from_size_align_unchecked(
                    self.element.size * self.capacity,
                    self.element.align,
                );
                std::alloc::dealloc(ptr, layout);
            }
        }
    }
}

impl Debug for TagStorage {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "TagStorage {{ element_size: {}, count: {}, capacity: {} }}",
            self.element.size, self.len, self.capacity
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::num::Wrapping;

    #[derive(Copy, Clone, PartialEq, Debug)]
    struct ZeroSize;

    #[test]
    pub fn create() {
        let _ = tracing_subscriber::fmt::try_init();

        let mut archetypes = Storage::new(WorldId::default());

        let mut desc = ArchetypeDescription::default();
        desc.register_tag::<usize>();
        desc.register_component::<isize>();

        let (_arch_id, data) = archetypes.alloc_archetype(desc);
        let set = data.alloc_chunk_set(|tags| unsafe {
            tags.get_mut(TagTypeId::of::<usize>()).unwrap().push(1isize)
        });

        let chunk_index = data.get_free_chunk(set, 1);
        let components = data
            .chunkset_mut(set)
            .unwrap()
            .chunk_mut(chunk_index)
            .unwrap();
        let mut writer = components.writer();
        let (chunk_entities, chunk_components) = writer.get();

        chunk_entities.push(Entity::new(1, Wrapping(0)));
        unsafe {
            (&mut *chunk_components.get())
                .get_mut(ComponentTypeId::of::<isize>())
                .unwrap()
                .writer()
                .push(&[1usize]);
        }
    }

    #[test]
    pub fn create_lazy_allocated() {
        let _ = tracing_subscriber::fmt::try_init();

        let mut archetypes = Storage::new(WorldId::default());

        let mut desc = ArchetypeDescription::default();
        desc.register_tag::<usize>();
        desc.register_component::<isize>();

        let (_arch_id, data) = archetypes.alloc_archetype(desc);
        let set = data.alloc_chunk_set(|tags| unsafe {
            tags.get_mut(TagTypeId::of::<usize>()).unwrap().push(1isize)
        });

        let chunk_index = data.get_free_chunk(set, 1);
        let chunk = data
            .chunkset_mut(set)
            .unwrap()
            .chunk_mut(chunk_index)
            .unwrap();

        assert!(!chunk.is_allocated());

        chunk.writer();

        assert!(chunk.is_allocated());
    }

    #[test]
    pub fn create_free_when_empty() {
        let _ = tracing_subscriber::fmt::try_init();

        let mut archetypes = Storage::new(WorldId::default());

        let mut desc = ArchetypeDescription::default();
        desc.register_tag::<usize>();
        desc.register_component::<isize>();

        let (_arch_id, data) = archetypes.alloc_archetype(desc);
        let set = data.alloc_chunk_set(|tags| unsafe {
            tags.get_mut(TagTypeId::of::<usize>()).unwrap().push(1isize)
        });

        let chunk_index = data.get_free_chunk(set, 1);
        let chunk = data
            .chunkset_mut(set)
            .unwrap()
            .chunk_mut(chunk_index)
            .unwrap();

        assert!(!chunk.is_allocated());

        {
            let mut writer = chunk.writer();
            let (chunk_entities, chunk_components) = writer.get();

            chunk_entities.push(Entity::new(1, Wrapping(0)));
            unsafe {
                (&mut *chunk_components.get())
                    .get_mut(ComponentTypeId::of::<isize>())
                    .unwrap()
                    .writer()
                    .push(&[1usize]);
            }
        }

        assert!(chunk.is_allocated());

        chunk.swap_remove(ComponentIndex(0), true);

        assert!(!chunk.is_allocated());
    }

    #[test]
    pub fn read_components() {
        let _ = tracing_subscriber::fmt::try_init();

        let mut archetypes = Storage::new(WorldId::default());

        let mut desc = ArchetypeDescription::default();
        desc.register_component::<isize>();
        desc.register_component::<usize>();
        desc.register_component::<ZeroSize>();

        let (_arch_id, data) = archetypes.alloc_archetype(desc);
        let set = data.alloc_chunk_set(|_| {});
        let chunk_index = data.get_free_chunk(set, 1);
        let components = data
            .chunkset_mut(set)
            .unwrap()
            .chunk_mut(chunk_index)
            .unwrap();

        let entities = [
            (Entity::new(1, Wrapping(0)), 1isize, 1usize, ZeroSize),
            (Entity::new(2, Wrapping(0)), 2isize, 2usize, ZeroSize),
            (Entity::new(3, Wrapping(0)), 3isize, 3usize, ZeroSize),
        ];

        let mut writer = components.writer();
        let (chunk_entities, chunk_components) = writer.get();
        for (entity, c1, c2, c3) in entities.iter() {
            chunk_entities.push(*entity);
            unsafe {
                (&mut *chunk_components.get())
                    .get_mut(ComponentTypeId::of::<isize>())
                    .unwrap()
                    .writer()
                    .push(&[*c1]);
                (&mut *chunk_components.get())
                    .get_mut(ComponentTypeId::of::<usize>())
                    .unwrap()
                    .writer()
                    .push(&[*c2]);
                (&mut *chunk_components.get())
                    .get_mut(ComponentTypeId::of::<ZeroSize>())
                    .unwrap()
                    .writer()
                    .push(&[*c3]);
            }
        }

        unsafe {
            for (i, c) in (*chunk_components.get())
                .get(ComponentTypeId::of::<isize>())
                .unwrap()
                .data_slice::<isize>()
                .iter()
                .enumerate()
            {
                assert_eq!(entities[i].1, *c);
            }

            for (i, c) in (*chunk_components.get())
                .get(ComponentTypeId::of::<usize>())
                .unwrap()
                .data_slice::<usize>()
                .iter()
                .enumerate()
            {
                assert_eq!(entities[i].2, *c);
            }

            for (i, c) in (*chunk_components.get())
                .get(ComponentTypeId::of::<ZeroSize>())
                .unwrap()
                .data_slice::<ZeroSize>()
                .iter()
                .enumerate()
            {
                assert_eq!(entities[i].3, *c);
            }
        }
    }

    #[test]
    pub fn read_tags() {
        let _ = tracing_subscriber::fmt::try_init();

        let mut archetypes = Storage::new(WorldId::default());

        let mut desc = ArchetypeDescription::default();
        desc.register_tag::<isize>();
        desc.register_tag::<ZeroSize>();

        let (_arch_id, data) = archetypes.alloc_archetype(desc);

        let tag_values = [(0isize, ZeroSize), (1isize, ZeroSize), (2isize, ZeroSize)];

        for (t1, t2) in tag_values.iter() {
            data.alloc_chunk_set(|tags| {
                unsafe { tags.get_mut(TagTypeId::of::<isize>()).unwrap().push(*t1) };
                unsafe { tags.get_mut(TagTypeId::of::<ZeroSize>()).unwrap().push(*t2) };
            });
        }

        unsafe {
            let tags1 = data
                .tags()
                .get(TagTypeId::of::<isize>())
                .unwrap()
                .data_slice::<isize>();
            assert_eq!(tags1.len(), tag_values.len());
            for (i, t) in tags1.iter().enumerate() {
                assert_eq!(tag_values[i].0, *t);
            }

            let tags2 = data
                .tags()
                .get(TagTypeId::of::<ZeroSize>())
                .unwrap()
                .data_slice::<ZeroSize>();
            assert_eq!(tags2.len(), tag_values.len());
            for (i, t) in tags2.iter().enumerate() {
                assert_eq!(tag_values[i].1, *t);
            }
        }
    }

    #[test]
    pub fn create_zero_size_tags() {
        let _ = tracing_subscriber::fmt::try_init();

        let mut archetypes = Storage::new(WorldId::default());

        let mut desc = ArchetypeDescription::default();
        desc.register_tag::<ZeroSize>();
        desc.register_component::<isize>();

        let (_arch_id, data) = archetypes.alloc_archetype(desc);
        let set = data.alloc_chunk_set(|tags| unsafe {
            tags.get_mut(TagTypeId::of::<ZeroSize>())
                .unwrap()
                .push(ZeroSize);
        });

        let chunk_index = data.get_free_chunk(set, 1);
        let components = data
            .chunkset_mut(set)
            .unwrap()
            .chunk_mut(chunk_index)
            .unwrap();
        let mut writer = components.writer();
        let (chunk_entities, chunk_components) = writer.get();

        chunk_entities.push(Entity::new(1, Wrapping(0)));
        unsafe {
            (&mut *chunk_components.get())
                .get_mut(ComponentTypeId::of::<isize>())
                .unwrap()
                .writer()
                .push(&[1usize]);
        }
    }

    #[test]
    pub fn create_zero_size_components() {
        let _ = tracing_subscriber::fmt::try_init();

        let mut archetypes = Storage::new(WorldId::default());

        let mut desc = ArchetypeDescription::default();
        desc.register_tag::<usize>();
        desc.register_component::<ZeroSize>();

        let (_arch_id, data) = archetypes.alloc_archetype(desc);
        let set = data.alloc_chunk_set(|tags| unsafe {
            tags.get_mut(TagTypeId::of::<usize>()).unwrap().push(1isize);
        });

        let chunk_index = data.get_free_chunk(set, 1);
        let components = data
            .chunkset_mut(set)
            .unwrap()
            .chunk_mut(chunk_index)
            .unwrap();
        let mut writer = components.writer();
        let (chunk_entities, chunk_components) = writer.get();

        chunk_entities.push(Entity::new(1, Wrapping(0)));
        unsafe {
            (&mut *chunk_components.get())
                .get_mut(ComponentTypeId::of::<ZeroSize>())
                .unwrap()
                .writer()
                .push(&[ZeroSize]);
        }
    }
}
