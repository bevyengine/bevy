//! A general-purpose allocator that manages a set of GPU buffer slabs.

use alloc::borrow::Cow;
use bevy_derive::{Deref, DerefMut};
use bevy_log::error;
use bevy_platform::collections::{hash_map::Entry, HashMap, HashSet};
use core::{
    cmp::Ordering,
    fmt::{self, Debug, Display, Formatter},
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::Range,
};
use nonmax::NonMaxU32;
use offset_allocator::{Allocation, Allocator};
use wgpu::{BufferDescriptor, BufferSize, BufferUsages, CommandEncoderDescriptor, WriteOnly};

use crate::{
    render_resource::Buffer,
    renderer::{RenderDevice, RenderQueue},
};

/// A general-purpose allocator that manages a set of GPU buffer slabs.
///
/// You can use this allocator to pack data that needs to be accessible by the
/// GPU into a small set of buffers, known as *slabs*. Each individual slab is
/// expected to contain homogeneous data of a single type. However, you can use
/// a single allocator to manage multiple slabs, each of which can have a
/// different data layout. Objects managed by the allocator are referenced with
/// a *key* that you can define.
///
/// To use this allocator, implement the [`SlabItem`] trait; see the
/// documentation of that trait for details.
///
/// For performance, you'll want to batch your allocation and deallocation
/// operations to be performed at a single point in the frame. To perform
/// allocation, call [`Self::stage_allocation`] to obtain an
/// [`AllocationStage`], call [`AllocationStage::allocate`] to allocate
/// individual objects, and then *commit* the allocation transaction using
/// [`AllocationStage::commit`]. Likewise, to perform deallocation, call
/// [`Self::stage_deallocation`] to obtain a [`DeallocationStage`], call
/// [`DeallocationStage::free`] to free objects, and then call
/// [`DeallocationStage::commit`]. Once you've committed an allocation stage,
/// you can copy new data into the slabs via [`Self::copy_element_data`].
///
/// Within each slab, or hardware buffer, the underlying allocation algorithm
/// is [`offset_allocator`], a Rust port of Sebastian Aaltonen's hard-real-time
/// C++ `OffsetAllocator`. Slabs start small and then grow as their contents
/// fill up, up to a maximum size limit. To reduce fragmentation, objects that
/// are too large bypass this system and receive their own buffers.
///
/// The [`SlabAllocatorSettings`] allows you to tune the behavior of the
/// allocator for better performance with your use case.
///
/// See [`crate::mesh::allocator::MeshAllocator`] for an example of usage.
pub struct SlabAllocator<I>
where
    I: SlabItem,
{
    /// Holds all buffers and allocators.
    pub slabs: HashMap<SlabId<I>, Slab<I>>,

    /// The next slab ID to assign.
    next_slab_id: SlabId<I>,

    /// Maps slab allocation keys to the ID of the slabs that hold their data.
    pub key_to_slab: HashMap<I::Key, SlabId<I>>,

    /// Maps a layout to the slabs that hold elements of that layout.
    ///
    /// This is used when allocating, so that we can find the appropriate slab
    /// to place an object in.
    slab_layouts: HashMap<I::Layout, Vec<SlabId<I>>>,

    /// Additional buffer usages to add to any vertex or index buffers created.
    pub extra_buffer_usages: BufferUsages,
}

/// Describes the type of the data that a [`SlabAllocator`] will store.
///
/// The actual type that you implement this trait on doesn't matter; only the
/// associated types [`Self::Key`] and [`Self::Layout`] do. Typically, you
/// implement this trait on a unit struct.
///
/// See [`crate::mesh::allocator::MeshSlabItem`] for an example of usage.
pub trait SlabItem {
    /// The key that's used to look up items in the allocator.
    type Key: Clone + PartialEq + Eq + Hash;

    /// A type that describes the layout of items within a single slab.
    ///
    /// If this slab allocator only allocates items of a single type, this type
    /// can simply be a unit struct. However, if you wish to have a single slab
    /// allocator that manages slabs of differing types, you can store metadata
    /// within values of this type that describes the size and alignment
    /// requirements of the objects within the slab. Each slab that the slab
    /// allocator manages contains an instance of this value so that it can
    /// track size and alignment requirements for that slab.
    type Layout: SlabItemLayout;

    /// Returns a suitable debugging label describing the type of elements that
    /// this slab item stores.
    fn label() -> Cow<'static, str>;
}

/// A trait that defines information necessary to determine the size and
/// alignment of objects within a slab.
pub trait SlabItemLayout: Clone + PartialEq + Eq + Hash {
    /// The size in bytes of a single element.
    ///
    /// This is the smallest size that this allocator can allocate, and all
    /// allocations must have a byte size that is a multiple of this value.
    fn size(&self) -> u64;

    /// The number of elements that make up a single slot.
    fn elements_per_slot(&self) -> u32;

    /// The `wgpu` buffer usages that the slab allocator will specify when
    /// creating buffers.
    ///
    /// `BufferUsages::COPY_DST` and `BufferUsages::COPY_SRC` are always
    /// included, regardless of what you specify here.
    fn buffer_usages(&self) -> BufferUsages;
}

/// Internal helper methods for [`SlabItemLayout`]s.
trait SlabItemLayoutExt {
    /// Returns the size in bytes of a single slot.
    fn slot_size(&self) -> u64;
}

impl<I> SlabItemLayoutExt for I
where
    I: SlabItemLayout,
{
    fn slot_size(&self) -> u64 {
        self.size() * self.elements_per_slot() as u64
    }
}

/// Tunable parameters that customize the behavior of the allocator.
///
/// Generally, these parameters adjust the tradeoff between memory fragmentation
/// and performance. You can adjust them as desired for your application. Most
/// applications can stick with the default values.
pub struct SlabAllocatorSettings {
    /// The minimum size of a slab (hardware buffer), in bytes.
    ///
    /// The default value is 1 MiB.
    pub min_slab_size: u64,

    /// The maximum size of a slab (hardware buffer), in bytes.
    ///
    /// When a slab reaches this limit, a new slab is created.
    ///
    /// The default value is 512 MiB.
    pub max_slab_size: u64,

    /// The maximum size of vertex or index data that can be placed in a general
    /// slab, in bytes.
    ///
    /// If an allocation exceeds this size limit, that data is placed in its own
    /// slab. This reduces fragmentation at the cost of more buffer management
    /// overhead.
    ///
    /// The default value is 256 MiB.
    pub large_threshold: u64,

    /// The factor by which we scale a slab when growing it.
    ///
    /// This value must be greater than 1. Higher values result in more
    /// fragmentation but fewer expensive copy operations when growing the
    /// buffer.
    ///
    /// The default value is 1.5.
    pub growth_factor: f64,
}

impl Default for SlabAllocatorSettings {
    fn default() -> Self {
        Self {
            // 1 MiB
            min_slab_size: 1024 * 1024,
            // 512 MiB
            max_slab_size: 1024 * 1024 * 512,
            // 256 MiB
            large_threshold: 1024 * 1024 * 256,
            // 1.5× growth
            growth_factor: 1.5,
        }
    }
}

/// The index of a single slab.
#[derive(Deref, DerefMut)]
#[repr(transparent)]
pub struct SlabId<I>
where
    I: SlabItem,
{
    /// A value that represents the ID of the slab.
    #[deref]
    pub id: NonMaxU32,
    phantom: PhantomData<I>,
}

impl<I> Clone for SlabId<I>
where
    I: SlabItem,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<I> Copy for SlabId<I> where I: SlabItem {}

impl<I> Default for SlabId<I>
where
    I: SlabItem,
{
    fn default() -> Self {
        SlabId {
            id: NonMaxU32::default(),
            phantom: PhantomData,
        }
    }
}

impl<I> PartialEq for SlabId<I>
where
    I: SlabItem,
{
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<I> Eq for SlabId<I> where I: SlabItem {}

impl<I> PartialOrd for SlabId<I>
where
    I: SlabItem,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<I> Ord for SlabId<I>
where
    I: SlabItem,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(other)
    }
}

impl<I> Hash for SlabId<I>
where
    I: SlabItem,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<I> Debug for SlabId<I>
where
    I: SlabItem,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("SlabId").field("id", &self.id).finish()
    }
}

/// Data for a single slab.
#[expect(
    clippy::large_enum_variant,
    reason = "See https://github.com/bevyengine/bevy/issues/19220"
)]
pub enum Slab<I>
where
    I: SlabItem,
{
    /// A slab that can contain multiple objects.
    General(GeneralSlab<I>),
    /// A slab that contains a single object.
    LargeObject(LargeObjectSlab<I>),
}

/// A resizable slab that can contain multiple objects.
///
/// This is the normal type of slab used for objects that are below the
/// [`SlabAllocatorSettings::large_threshold`]. Slabs are divided into *slots*,
/// which are described in detail in the [`SlabItemLayout`] documentation.
pub struct GeneralSlab<I>
where
    I: SlabItem,
{
    /// The [`Allocator`] that manages the objects in this slab.
    allocator: Allocator,

    /// The GPU buffer that backs this slab.
    ///
    /// This may be `None` if the buffer hasn't been created yet. We delay
    /// creation of buffers until performing all the allocations for a single
    /// frame, so that we don't needlessly create and resize buffers when many
    /// objects are allocated all at once.
    buffer: Option<Buffer>,

    /// Allocations that are on the GPU.
    ///
    /// The range is in slots.
    resident_allocations: HashMap<I::Key, SlabAllocation>,

    /// Allocations that are waiting to be uploaded to the GPU.
    ///
    /// The range is in slots.
    pending_allocations: HashMap<I::Key, SlabAllocation>,

    /// The layout of a single element (vertex or index).
    element_layout: I::Layout,

    /// The size of this slab in slots.
    current_slot_capacity: u32,
}

/// A slab that contains a single object.
///
/// Typically, this is for objects that exceed the
/// [`SlabAllocatorSettings::large_threshold`]. Additionally, some uses of the
/// slab allocator may wish to force objects to possess their own slab. For
/// instance, due to platform limitations (vertex arrays on WebGL 2), the mesh
/// allocator sometimes needs to place meshes that would otherwise be allocated
/// together with other meshes in their own slab.
pub struct LargeObjectSlab<I>
where
    I: SlabItem,
{
    /// The GPU buffer that backs this slab.
    ///
    /// This may be `None` if the buffer hasn't been created yet.
    buffer: Option<Buffer>,

    /// The layout of a single element (vertex or index).
    element_layout: I::Layout,
}

/// The location of an allocation and the slab it's contained in.
struct SlabItemAllocation<I>
where
    I: SlabItem,
{
    /// The ID of the slab.
    slab_id: SlabId<I>,
    /// Holds the actual allocation.
    slab_allocation: SlabAllocation,
}

impl<I> Slab<I>
where
    I: SlabItem,
{
    /// Returns the GPU buffer corresponding to this slab, if it's been
    /// uploaded.
    pub fn buffer(&self) -> Option<&Buffer> {
        match self {
            Slab::General(general_slab) => general_slab.buffer.as_ref(),
            Slab::LargeObject(large_object_slab) => large_object_slab.buffer.as_ref(),
        }
    }

    /// Returns the size of this slab in bytes.
    pub fn buffer_size(&self) -> u64 {
        match self.buffer() {
            Some(buffer) => buffer.size(),
            None => 0,
        }
    }

    /// Returns the [`SlabItemLayout`] associated with this slab.
    pub fn element_layout(&self) -> &I::Layout {
        match self {
            Slab::General(general_slab) => &general_slab.element_layout,
            Slab::LargeObject(large_object_slab) => &large_object_slab.element_layout,
        }
    }
}

/// An object that allows batched allocation.
///
/// In order to perform allocations, you create one of these objects with
/// [`SlabAllocator::stage_allocation`], allocate into it with
/// [`Self::allocate`], and finally commit it with [`Self::commit`]. Always
/// make sure to call [`Self::commit`]; if you don't, buffers that were
/// supposed to be enlarged won't be.
pub struct AllocationStage<'a, I>
where
    I: SlabItem,
{
    /// The allocator that we're allocating objects into.
    pub allocator: &'a mut SlabAllocator<I>,
    /// The set of slabs that have grown and need to be reallocated.
    slabs_to_reallocate: HashMap<SlabId<I>, SlabToReallocate>,
}

impl<'a, I> Drop for AllocationStage<'a, I>
where
    I: SlabItem,
{
    fn drop(&mut self) {
        if !self.slabs_to_reallocate.is_empty() {
            error!(
                "Dropping an `AllocationStage` with uncommitted reallocations. You should call \
                `AllocationStage::commit`."
            );
        }
    }
}

impl<'a, I> AllocationStage<'a, I>
where
    I: SlabItem,
{
    /// Allocates space for an object of the given size with the given key and layout.
    ///
    /// The key must not correspond to any current allocation.
    pub fn allocate(
        &mut self,
        key: &I::Key,
        data_byte_len: u64,
        layout: I::Layout,
        settings: &SlabAllocatorSettings,
    ) {
        self.allocator.allocate(
            key,
            data_byte_len,
            layout,
            &mut self.slabs_to_reallocate,
            settings,
        );
    }

    /// Allocates an object into its own dedicated slab.
    ///
    /// The key must not correspond to any current allocation.
    pub fn allocate_large(&mut self, key: &I::Key, layout: I::Layout) {
        self.allocator.allocate_large(key, layout);
    }

    /// Completes the transaction, performing any queued resize operations.
    pub fn commit(mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        for (slab_id, slab_to_grow) in self.slabs_to_reallocate.drain() {
            self.allocator
                .reallocate_slab(render_device, render_queue, slab_id, slab_to_grow);
        }
    }
}

/// An object that enables batched deallocation.
///
/// To free objects from a [`SlabAllocator`], call
/// [`SlabAllocator::stage_deallocation`] to create a [`DeallocationStage`],
/// call [`Self::free`] to deallocate objects, and finally call
/// [`Self::commit`]. You must call [`Self::commit`] in order to ensure that
/// newly-empty slabs are deallocated.
pub struct DeallocationStage<'a, I>
where
    I: SlabItem,
{
    /// The allocator in which objects are to be freed.
    pub allocator: &'a mut SlabAllocator<I>,
    /// IDs of slabs that have become empty.
    empty_slabs: HashSet<SlabId<I>>,
}

impl<'a, I> Drop for DeallocationStage<'a, I>
where
    I: SlabItem,
{
    fn drop(&mut self) {
        if !self.empty_slabs.is_empty() {
            error!(
                "Dropping a `DeallocationStage` with uncommitted slab free operations. You should \
                call `DeallocationStage::commit`."
            );
        }
    }
}

impl<'a, I> DeallocationStage<'a, I>
where
    I: SlabItem,
{
    /// Schedules a free operation for the allocation with the given key.
    ///
    /// The key must correspond to a live allocation. An error will be emitted
    /// to the log otherwise.
    pub fn free(&mut self, key: &I::Key) {
        if let Some(slab_id) = self.allocator.key_to_slab.remove(key) {
            self.allocator
                .free_allocation_in_slab(key, slab_id, &mut self.empty_slabs);
        }
    }

    /// Performs all the free operations.
    ///
    /// You must call this method if you called [`Self::free`].
    pub fn commit(mut self) {
        self.allocator.free_empty_slabs(self.empty_slabs.drain());
    }
}

/// An allocation within a slab.
#[derive(Clone)]
struct SlabAllocation {
    /// The actual [`Allocator`] handle, needed to free the allocation.
    allocation: Allocation,
    /// The number of slots that this allocation takes up.
    slot_count: u32,
    /// The number of slots at the end of the allocation that are considered
    /// padding.
    padding: u32,
}

/// The hardware buffer that slab-allocated data lives in, as well as the range
/// within that buffer.
pub struct SlabAllocationBufferSlice<'a, I>
where
    I: SlabItem,
{
    /// The buffer that the data resides in.
    pub buffer: &'a Buffer,

    /// The range of elements within this buffer that the data resides in,
    /// measured in elements.
    ///
    /// This is an element range, not a byte range. For vertex data, this is
    /// measured in increments of a single vertex. (Thus, if a vertex is 32
    /// bytes long, then this range is in units of 32 bytes each.) For index
    /// data, this is measured in increments of a single index value (2 or 4
    /// bytes). Draw commands generally take their ranges in elements, not
    /// bytes, so this is the most convenient unit in this case.
    pub range: Range<u32>,

    phantom: PhantomData<I>,
}

/// Holds information about a slab that's scheduled to be allocated or
/// reallocated.
#[derive(Default)]
pub struct SlabToReallocate {
    /// The capacity of the slab before we decided to grow it.
    old_slot_capacity: u32,
}

impl<I> Display for SlabId<I>
where
    I: SlabItem,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.id, f)
    }
}

impl<I> Default for SlabAllocator<I>
where
    I: SlabItem,
{
    fn default() -> Self {
        Self {
            slabs: HashMap::default(),
            next_slab_id: SlabId {
                id: NonMaxU32::default(),
                phantom: PhantomData,
            },
            key_to_slab: HashMap::default(),
            slab_layouts: HashMap::default(),
            extra_buffer_usages: BufferUsages::empty(),
        }
    }
}

impl<I> SlabAllocator<I>
where
    I: SlabItem,
{
    /// Creates a new empty slab allocator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an [`AllocationStage`], enabling batched allocation of objects
    /// in this slab.
    ///
    /// Allocation of objects in the slab requires calling this function,
    /// calling [`AllocationStage::allocate`] on the resulting
    /// [`AllocationStage`], and finally calling [`AllocationStage::commit`].
    /// Grouping allocations into a batch, preferably at most one per frame, is
    /// the most efficient way to perform many allocations at once.
    pub fn stage_allocation(&'_ mut self) -> AllocationStage<'_, I> {
        AllocationStage {
            allocator: self,
            slabs_to_reallocate: HashMap::default(),
        }
    }

    /// Creates a [`DeallocationStage`], enabling batched deallocation.
    ///
    /// Deallocation of objects in the slab requires calling this function,
    /// calling [`DeallocationStage::free`] on the resulting
    /// [`DeallocationStage`], and finally calling
    /// [`DeallocationStage::commit`]. Grouping deallocations into a batch,
    /// preferably at most one per frame, is the most efficient way to perform
    /// many deallocations at once.
    pub fn stage_deallocation(&'_ mut self) -> DeallocationStage<'_, I> {
        DeallocationStage {
            allocator: self,
            empty_slabs: HashSet::default(),
        }
    }

    /// Allocates space for data with the given byte size and layout in the
    /// appropriate slab, creating that slab if necessary.
    fn allocate(
        &mut self,
        key: &I::Key,
        data_byte_len: u64,
        layout: I::Layout,
        slabs_to_grow: &mut HashMap<SlabId<I>, SlabToReallocate>,
        settings: &SlabAllocatorSettings,
    ) {
        debug_assert!(!self.key_to_slab.contains_key(key));

        let data_element_count = data_byte_len.div_ceil(layout.size()) as u32;
        let data_slot_count = data_element_count.div_ceil(layout.elements_per_slot());
        let padding = data_slot_count * layout.elements_per_slot() - data_element_count;

        // If the data is too large for a slab, give it a slab of its own.
        if data_slot_count as u64 * layout.slot_size()
            >= settings.large_threshold.min(settings.max_slab_size)
        {
            self.allocate_large(key, layout);
        } else {
            self.allocate_general(
                key,
                data_slot_count,
                padding,
                layout,
                slabs_to_grow,
                settings,
            );
        }
    }

    /// Allocates space for data with the given slot size and layout in the
    /// appropriate general slab.
    fn allocate_general(
        &mut self,
        key: &I::Key,
        data_slot_count: u32,
        padding: u32,
        layout: I::Layout,
        slabs_to_grow: &mut HashMap<SlabId<I>, SlabToReallocate>,
        settings: &SlabAllocatorSettings,
    ) {
        let candidate_slabs = self.slab_layouts.entry(layout.clone()).or_default();

        // Loop through the slabs that accept elements of the appropriate type
        // and try to allocate the data inside them. We go with the first one
        // that succeeds.
        let mut data_allocation = None;
        for &slab_id in &*candidate_slabs {
            let Some(Slab::General(slab)) = self.slabs.get_mut(&slab_id) else {
                unreachable!("Slab not found")
            };

            let Some(allocation) = slab.allocator.allocate(data_slot_count) else {
                continue;
            };

            // Try to fit the object in the slab, growing if necessary.
            match slab.grow_if_necessary(allocation.offset + data_slot_count, settings) {
                SlabGrowthResult::NoGrowthNeeded => {}
                SlabGrowthResult::NeededGrowth(slab_to_reallocate) => {
                    // If we already grew the slab this frame, don't replace the
                    // `SlabToReallocate` entry. We want to keep the entry
                    // corresponding to the size that the slab had at the start
                    // of the frame, so that we can copy only the used portion
                    // of the initial buffer to the new one.
                    if let Entry::Vacant(vacant_entry) = slabs_to_grow.entry(slab_id) {
                        vacant_entry.insert(slab_to_reallocate);
                    }
                }
                SlabGrowthResult::CantGrow => continue,
            }

            data_allocation = Some(SlabItemAllocation {
                slab_id,
                slab_allocation: SlabAllocation {
                    allocation,
                    slot_count: data_slot_count,
                    padding,
                },
            });
            break;
        }

        // If we still have no allocation, make a new slab.
        if data_allocation.is_none() {
            let new_slab_id = self.next_slab_id;
            self.next_slab_id.id =
                NonMaxU32::new(self.next_slab_id.id.get() + 1).unwrap_or_default();

            let new_slab = GeneralSlab::new(
                new_slab_id,
                &mut data_allocation,
                settings,
                layout,
                data_slot_count,
                padding,
            );

            self.slabs.insert(new_slab_id, Slab::General(new_slab));
            candidate_slabs.push(new_slab_id);
            slabs_to_grow.insert(new_slab_id, SlabToReallocate::default());
        }

        let data_allocation = data_allocation.expect("Should have been able to allocate");

        // Mark the allocation as pending. Don't copy it in just yet; further
        // data loaded this frame may result in its final allocation location
        // changing.
        if let Some(Slab::General(general_slab)) = self.slabs.get_mut(&data_allocation.slab_id) {
            general_slab
                .pending_allocations
                .insert(key.clone(), data_allocation.slab_allocation);
        };

        self.record_allocation(key, data_allocation.slab_id);
    }

    /// Allocates an object into its own dedicated slab.
    fn allocate_large(&mut self, key: &I::Key, layout: I::Layout) {
        let new_slab_id = self.next_slab_id;
        self.next_slab_id.id = NonMaxU32::new(self.next_slab_id.id.get() + 1).unwrap_or_default();

        self.record_allocation(key, new_slab_id);

        self.slabs.insert(
            new_slab_id,
            Slab::LargeObject(LargeObjectSlab {
                buffer: None,
                element_layout: layout,
            }),
        );
    }

    /// Given a slab and the key corresponding to an object within it, marks
    /// the allocation as free.
    ///
    /// If this results in the slab becoming empty, this function adds the slab
    /// to the `empty_slabs` set.
    fn free_allocation_in_slab(
        &mut self,
        key: &I::Key,
        slab_id: SlabId<I>,
        empty_slabs: &mut HashSet<SlabId<I>>,
    ) {
        let Some(slab) = self.slabs.get_mut(&slab_id) else {
            error!("Double free: attempted to free data in a nonexistent slab");
            return;
        };

        match *slab {
            Slab::General(ref mut general_slab) => {
                let Some(slab_allocation) = general_slab
                    .resident_allocations
                    .remove(key)
                    .or_else(|| general_slab.pending_allocations.remove(key))
                else {
                    return;
                };

                general_slab.allocator.free(slab_allocation.allocation);

                if general_slab.is_empty() {
                    empty_slabs.insert(slab_id);
                }
            }
            Slab::LargeObject(_) => {
                empty_slabs.insert(slab_id);
            }
        }
    }

    /// Reallocates a slab that needs to be resized, or allocates a new slab.
    ///
    /// This performs the actual growth operation that
    /// [`GeneralSlab::grow_if_necessary`] scheduled. We do the growth in two
    /// phases so that, if a slab grows multiple times in the same frame, only
    /// one new buffer is reallocated, rather than reallocating the buffer
    /// multiple times.
    fn reallocate_slab(
        &mut self,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
        slab_id: SlabId<I>,
        slab_to_grow: SlabToReallocate,
    ) {
        let Some(Slab::General(slab)) = self.slabs.get_mut(&slab_id) else {
            error!("Couldn't find slab {} to grow", slab_id);
            return;
        };

        let old_buffer = slab.buffer.take();

        let buffer_usages =
            BufferUsages::COPY_SRC | BufferUsages::COPY_DST | slab.element_layout.buffer_usages();

        // Create the buffer.
        let new_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some(&format!(
                "general {} slab {} ({}buffer)",
                I::label(),
                slab_id,
                buffer_usages_to_str(buffer_usages)
            )),
            size: slab.current_slot_capacity as u64 * slab.element_layout.slot_size(),
            usage: buffer_usages | self.extra_buffer_usages,
            mapped_at_creation: false,
        });

        slab.buffer = Some(new_buffer.clone());

        let Some(old_buffer) = old_buffer else { return };

        // In order to do buffer copies, we need a command encoder.
        let mut encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some(&*format!("{} slab resize encoder", I::label())),
        });

        // Copy the data from the old buffer into the new one.
        encoder.copy_buffer_to_buffer(
            &old_buffer,
            0,
            &new_buffer,
            0,
            slab_to_grow.old_slot_capacity as u64 * slab.element_layout.slot_size(),
        );

        let command_buffer = encoder.finish();
        render_queue.submit([command_buffer]);
    }

    /// Records the location of the given newly-allocated data in the
    /// [`Self::key_to_slab`] table.
    fn record_allocation(&mut self, key: &I::Key, slab_id: SlabId<I>) {
        self.key_to_slab.insert(key.clone(), slab_id);
    }

    /// Returns the GPU buffer corresponding to the slab with the given ID if
    /// that slab has been uploaded to the GPU.
    pub fn buffer_for_slab(&self, slab_id: SlabId<I>) -> Option<&Buffer> {
        self.slabs.get(&slab_id).and_then(|slab| slab.buffer())
    }

    /// Given a slab and the key of data located with it, returns the buffer
    /// and range of that data within the slab.
    pub fn slab_allocation_slice(
        &self,
        key: &I::Key,
        slab_id: SlabId<I>,
    ) -> Option<SlabAllocationBufferSlice<'_, I>> {
        match self.slabs.get(&slab_id)? {
            Slab::General(general_slab) => {
                let slab_allocation = general_slab.resident_allocations.get(key)?;
                Some(SlabAllocationBufferSlice {
                    buffer: general_slab.buffer.as_ref()?,
                    range: (slab_allocation.allocation.offset
                        * general_slab.element_layout.elements_per_slot())
                        ..((slab_allocation.allocation.offset + slab_allocation.slot_count)
                            * general_slab.element_layout.elements_per_slot())
                            - slab_allocation.padding,
                    phantom: PhantomData,
                })
            }

            Slab::LargeObject(large_object_slab) => {
                let buffer = large_object_slab.buffer.as_ref()?;
                Some(SlabAllocationBufferSlice {
                    buffer,
                    range: 0..((buffer.size() / large_object_slab.element_layout.size()) as u32),
                    phantom: PhantomData,
                })
            }
        }
    }

    fn free_empty_slabs(&mut self, empty_slabs: impl Iterator<Item = SlabId<I>>) {
        for empty_slab in empty_slabs {
            self.slab_layouts.values_mut().for_each(|slab_ids| {
                let idx = slab_ids.iter().position(|&slab_id| slab_id == empty_slab);
                if let Some(idx) = idx {
                    slab_ids.remove(idx);
                }
            });
            self.slabs.remove(&empty_slab);
        }
    }

    /// Get the number of allocated slabs
    pub fn slab_count(&self) -> usize {
        self.slabs.len()
    }

    /// Get the total size of all allocated slabs
    pub fn slabs_size(&self) -> u64 {
        self.slabs.iter().map(|slab| slab.1.buffer_size()).sum()
    }

    /// Copies data into an allocated slab.
    ///
    /// `len` specifies the size of the data to be copied *in bytes*. The given
    /// `fill_data` callback is expected to write the data into the given slice;
    /// this callback approach avoids a copy.
    pub fn copy_element_data(
        &mut self,
        key: &I::Key,
        len: usize,
        fill_data: impl Fn(WriteOnly<[u8]>),
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) {
        let Some(slab_id) = self.key_to_slab.get(key) else {
            error!("Use-after-free: attempted to copy element data for an unallocated key");
            return;
        };
        let Some(slab) = self.slabs.get_mut(slab_id) else {
            error!("Use-after-free: attempted to copy element data into a nonexistent slab");
            return;
        };

        match *slab {
            Slab::General(ref mut general_slab) => {
                let (Some(buffer), Some(allocated_range)) = (
                    &general_slab.buffer,
                    general_slab.pending_allocations.remove(key),
                ) else {
                    return;
                };

                let slot_size = general_slab.element_layout.slot_size();

                // round up size to a multiple of the slot size to satisfy wgpu
                // alignment requirements
                if let Some(size) = BufferSize::new((len as u64).next_multiple_of(slot_size)) {
                    // Write the data in.
                    if let Some(mut buffer) = render_queue.write_buffer_with(
                        buffer,
                        allocated_range.allocation.offset as u64 * slot_size,
                        size,
                    ) {
                        let slice = buffer.slice(..len);
                        fill_data(slice);
                    }
                }

                // Mark the allocation as resident.
                general_slab
                    .resident_allocations
                    .insert(key.clone(), allocated_range);
            }

            Slab::LargeObject(ref mut large_object_slab) => {
                debug_assert!(large_object_slab.buffer.is_none());

                // Create the buffer and its data in one go.
                let buffer_usages = large_object_slab.element_layout.buffer_usages();
                let buffer = render_device.create_buffer(&BufferDescriptor {
                    label: Some(&format!(
                        "large {} slab {} ({}buffer)",
                        I::label(),
                        slab_id,
                        buffer_usages_to_str(buffer_usages)
                    )),
                    size: len as u64,
                    usage: buffer_usages | BufferUsages::COPY_DST,
                    mapped_at_creation: true,
                });
                {
                    let mut slice = buffer.slice(..).get_mapped_range_mut();

                    fill_data(slice.slice(..len));
                }
                buffer.unmap();
                large_object_slab.buffer = Some(buffer);
            }
        }
    }
}

/// The results of [`GeneralSlab::grow_if_necessary`].
enum SlabGrowthResult {
    /// The data already fits in the slab; the slab doesn't need to grow.
    NoGrowthNeeded,
    /// The slab needed to grow.
    ///
    /// The [`SlabToReallocate`] contains the old capacity of the slab.
    NeededGrowth(SlabToReallocate),
    /// The slab wanted to grow but couldn't because it hit its maximum size.
    CantGrow,
}

impl<I> GeneralSlab<I>
where
    I: SlabItem,
{
    /// Creates a new growable slab big enough to hold a single element of
    /// `data_slot_count` size with the given `layout`.
    fn new(
        new_slab_id: SlabId<I>,
        maybe_slab_item_allocation: &mut Option<SlabItemAllocation<I>>,
        settings: &SlabAllocatorSettings,
        layout: I::Layout,
        data_slot_count: u32,
        padding: u32,
    ) -> GeneralSlab<I> {
        let initial_slab_slot_capacity = (settings.min_slab_size.div_ceil(layout.slot_size())
            as u32)
            .max(offset_allocator::ext::min_allocator_size(data_slot_count));
        let max_slab_slot_capacity = (settings.max_slab_size.div_ceil(layout.slot_size()) as u32)
            .max(offset_allocator::ext::min_allocator_size(data_slot_count));

        let mut new_slab = GeneralSlab {
            allocator: Allocator::new(max_slab_slot_capacity),
            buffer: None,
            resident_allocations: HashMap::default(),
            pending_allocations: HashMap::default(),
            element_layout: layout,
            current_slot_capacity: initial_slab_slot_capacity,
        };

        // This should never fail.
        if let Some(allocation) = new_slab.allocator.allocate(data_slot_count) {
            *maybe_slab_item_allocation = Some(SlabItemAllocation {
                slab_id: new_slab_id,
                slab_allocation: SlabAllocation {
                    slot_count: data_slot_count,
                    allocation,
                    padding,
                },
            });
        }

        new_slab
    }

    /// Checks to see if the size of this slab is at least `new_size_in_slots`
    /// and grows the slab if it isn't.
    ///
    /// The returned [`SlabGrowthResult`] describes whether the slab needed to
    /// grow and whether, if so, it was successful in doing so.
    fn grow_if_necessary(
        &mut self,
        new_size_in_slots: u32,
        settings: &SlabAllocatorSettings,
    ) -> SlabGrowthResult {
        // Is the slab big enough already?
        let initial_slot_capacity = self.current_slot_capacity;
        if self.current_slot_capacity >= new_size_in_slots {
            return SlabGrowthResult::NoGrowthNeeded;
        }

        // Try to grow in increments of `SlabAllocatorSettings::growth_factor`
        // until we're big enough.
        while self.current_slot_capacity < new_size_in_slots {
            let new_slab_slot_capacity =
                ((self.current_slot_capacity as f64 * settings.growth_factor).ceil() as u32)
                    .min((settings.max_slab_size / self.element_layout.slot_size()) as u32);
            if new_slab_slot_capacity == self.current_slot_capacity {
                // The slab is full.
                return SlabGrowthResult::CantGrow;
            }

            self.current_slot_capacity = new_slab_slot_capacity;
        }

        // Tell our caller what we did.
        SlabGrowthResult::NeededGrowth(SlabToReallocate {
            old_slot_capacity: initial_slot_capacity,
        })
    }

    /// Returns true if this slab is empty.
    fn is_empty(&self) -> bool {
        self.resident_allocations.is_empty() && self.pending_allocations.is_empty()
    }
}

/// Returns a string describing the given buffer usages.
fn buffer_usages_to_str(buffer_usages: BufferUsages) -> &'static str {
    if buffer_usages.contains(BufferUsages::VERTEX) {
        "vertex "
    } else if buffer_usages.contains(BufferUsages::INDEX) {
        "index "
    } else if buffer_usages.contains(BufferUsages::STORAGE) {
        "storage "
    } else {
        ""
    }
}
