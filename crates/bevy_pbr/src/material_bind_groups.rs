//! Material bind group management for bindless resources.
//!
//! In bindless mode, Bevy's renderer groups materials into bind groups. This
//! allocator manages each bind group, assigning slots to materials as
//! appropriate.

use core::{cmp::Ordering, iter, marker::PhantomData, mem, ops::Range};

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    resource::Resource,
    world::{FromWorld, World},
};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_render::{
    render_resource::{
        BindGroup, BindGroupEntry, BindGroupLayout, BindingNumber, BindingResource,
        BindingResources, BindlessDescriptor, BindlessIndex, BindlessIndexTableDescriptor,
        BindlessResourceType, Buffer, BufferBinding, BufferDescriptor, BufferId,
        BufferInitDescriptor, BufferUsages, CompareFunction, FilterMode, OwnedBindingResource,
        PreparedBindGroup, RawBufferVec, Sampler, SamplerDescriptor, SamplerId, TextureView,
        TextureViewDimension, TextureViewId, UnpreparedBindGroup, WgpuSampler, WgpuTextureView,
    },
    renderer::{RenderDevice, RenderQueue},
    settings::WgpuFeatures,
    texture::FallbackImage,
};
use bevy_utils::default;
use bytemuck::Pod;
use tracing::{error, trace};

use crate::Material;

/// A resource that places materials into bind groups and tracks their
/// resources.
///
/// Internally, Bevy has separate allocators for bindless and non-bindless
/// materials. This resource provides a common interface to the specific
/// allocator in use.
#[derive(Resource)]
pub enum MaterialBindGroupAllocator<M>
where
    M: Material,
{
    /// The allocator used when the material is bindless.
    Bindless(Box<MaterialBindGroupBindlessAllocator<M>>),
    /// The allocator used when the material is non-bindless.
    NonBindless(Box<MaterialBindGroupNonBindlessAllocator<M>>),
}

/// The allocator that places bindless materials into bind groups and tracks
/// their resources.
pub struct MaterialBindGroupBindlessAllocator<M>
where
    M: Material,
{
    /// The slabs, each of which contains a bind group.
    slabs: Vec<MaterialBindlessSlab<M>>,
    /// The layout of the bind groups that we produce.
    bind_group_layout: BindGroupLayout,
    /// Information about the bindless resources in the material.
    ///
    /// We use this information to create and maintain bind groups.
    bindless_descriptor: BindlessDescriptor,

    /// Dummy buffers that we use to fill empty slots in buffer binding arrays.
    ///
    /// There's one fallback buffer for each buffer in the bind group, each
    /// appropriately sized. Each buffer contains one uninitialized element of
    /// the applicable type.
    fallback_buffers: HashMap<BindlessIndex, Buffer>,

    /// The maximum number of resources that can be stored in a slab.
    ///
    /// This corresponds to `SLAB_CAPACITY` in the `#[bindless(SLAB_CAPACITY)]`
    /// attribute, when deriving `AsBindGroup`.
    slab_capacity: u32,
}

/// A single bind group and the bookkeeping necessary to allocate into it.
pub struct MaterialBindlessSlab<M>
where
    M: Material,
{
    /// The current bind group, if it's up to date.
    ///
    /// If this is `None`, then the bind group is dirty and needs to be
    /// regenerated.
    bind_group: Option<BindGroup>,

    /// The GPU-accessible buffers that hold the mapping from binding index to
    /// bindless slot.
    ///
    /// This is conventionally assigned to bind group binding 0, but it can be
    /// changed using the `#[bindless(index_table(binding(B)))]` attribute on
    /// `AsBindGroup`.
    ///
    /// Because the slab binary searches this table, the entries within must be
    /// sorted by bindless index.
    bindless_index_tables: Vec<MaterialBindlessIndexTable<M>>,

    /// The binding arrays containing samplers.
    samplers: HashMap<BindlessResourceType, MaterialBindlessBindingArray<Sampler>>,
    /// The binding arrays containing textures.
    textures: HashMap<BindlessResourceType, MaterialBindlessBindingArray<TextureView>>,
    /// The binding arrays containing buffers.
    buffers: HashMap<BindlessIndex, MaterialBindlessBindingArray<Buffer>>,
    /// The buffers that contain plain old data (i.e. the structure-level
    /// `#[data]` attribute of `AsBindGroup`).
    data_buffers: HashMap<BindlessIndex, MaterialDataBuffer>,

    /// Holds extra CPU-accessible data that the material provides.
    ///
    /// Typically, this data is used for constructing the material key, for
    /// pipeline specialization purposes.
    extra_data: Vec<Option<M::Data>>,

    /// A list of free slot IDs.
    free_slots: Vec<MaterialBindGroupSlot>,
    /// The total number of materials currently allocated in this slab.
    live_allocation_count: u32,
    /// The total number of resources currently allocated in the binding arrays.
    allocated_resource_count: u32,
}

/// A GPU-accessible buffer that holds the mapping from binding index to
/// bindless slot.
///
/// This is conventionally assigned to bind group binding 0, but it can be
/// changed by altering the [`Self::binding_number`], which corresponds to the
/// `#[bindless(index_table(binding(B)))]` attribute in `AsBindGroup`.
struct MaterialBindlessIndexTable<M>
where
    M: Material,
{
    /// The buffer containing the mappings.
    buffer: RetainedRawBufferVec<u32>,
    /// The range of bindless indices that this bindless index table covers.
    ///
    /// If this range is M..N, then the field at index $i$ maps to bindless
    /// index $i$ + M. The size of this table is N - M.
    ///
    /// This corresponds to the `#[bindless(index_table(range(M..N)))]`
    /// attribute in `AsBindGroup`.
    index_range: Range<BindlessIndex>,
    /// The binding number that this index table is assigned to in the shader.
    binding_number: BindingNumber,
    phantom: PhantomData<M>,
}

/// A single binding array for storing bindless resources and the bookkeeping
/// necessary to allocate into it.
struct MaterialBindlessBindingArray<R>
where
    R: GetBindingResourceId,
{
    /// The number of the binding that we attach this binding array to.
    binding_number: BindingNumber,
    /// A mapping from bindless slot index to the resource stored in that slot,
    /// if any.
    bindings: Vec<Option<MaterialBindlessBinding<R>>>,
    /// The type of resource stored in this binding array.
    resource_type: BindlessResourceType,
    /// Maps a resource ID to the slot in which it's stored.
    ///
    /// This is essentially the inverse mapping of [`Self::bindings`].
    resource_to_slot: HashMap<BindingResourceId, u32>,
    /// A list of free slots in [`Self::bindings`] that contain no binding.
    free_slots: Vec<u32>,
    /// The number of allocated objects in this binding array.
    len: u32,
}

/// A single resource (sampler, texture, or buffer) in a binding array.
///
/// Resources hold a reference count, which specifies the number of materials
/// currently allocated within the slab that refer to this resource. When the
/// reference count drops to zero, the resource is freed.
struct MaterialBindlessBinding<R>
where
    R: GetBindingResourceId,
{
    /// The sampler, texture, or buffer.
    resource: R,
    /// The number of materials currently allocated within the containing slab
    /// that use this resource.
    ref_count: u32,
}

/// The allocator that stores bind groups for non-bindless materials.
pub struct MaterialBindGroupNonBindlessAllocator<M>
where
    M: Material,
{
    /// A mapping from [`MaterialBindGroupIndex`] to the bind group allocated in
    /// each slot.
    bind_groups: Vec<Option<MaterialNonBindlessAllocatedBindGroup<M>>>,
    /// The bind groups that are dirty and need to be prepared.
    ///
    /// To prepare the bind groups, call
    /// [`MaterialBindGroupAllocator::prepare_bind_groups`].
    to_prepare: HashSet<MaterialBindGroupIndex>,
    /// A list of free bind group indices.
    free_indices: Vec<MaterialBindGroupIndex>,
    phantom: PhantomData<M>,
}

/// A single bind group that a [`MaterialBindGroupNonBindlessAllocator`] is
/// currently managing.
enum MaterialNonBindlessAllocatedBindGroup<M>
where
    M: Material,
{
    /// An unprepared bind group.
    ///
    /// The allocator prepares all outstanding unprepared bind groups when
    /// [`MaterialBindGroupNonBindlessAllocator::prepare_bind_groups`] is
    /// called.
    Unprepared {
        /// The unprepared bind group, including extra data.
        bind_group: UnpreparedBindGroup<M::Data>,
        /// The layout of that bind group.
        layout: BindGroupLayout,
    },
    /// A bind group that's already been prepared.
    Prepared {
        bind_group: PreparedBindGroup<M::Data>,
        #[expect(dead_code, reason = "These buffers are only referenced by bind groups")]
        uniform_buffers: Vec<Buffer>,
    },
}

/// Dummy instances of various resources that we fill unused slots in binding
/// arrays with.
#[derive(Resource)]
pub struct FallbackBindlessResources {
    /// A dummy filtering sampler.
    filtering_sampler: Sampler,
    /// A dummy non-filtering sampler.
    non_filtering_sampler: Sampler,
    /// A dummy comparison sampler.
    comparison_sampler: Sampler,
}

/// The `wgpu` ID of a single bindless or non-bindless resource.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum BindingResourceId {
    /// A buffer.
    Buffer(BufferId),
    /// A texture view, with the given dimension.
    TextureView(TextureViewDimension, TextureViewId),
    /// A sampler.
    Sampler(SamplerId),
    /// A buffer containing plain old data.
    ///
    /// This corresponds to the `#[data]` structure-level attribute on
    /// `AsBindGroup`.
    DataBuffer,
}

/// A temporary list of references to `wgpu` bindless resources.
///
/// We need this because the `wgpu` bindless API takes a slice of references.
/// Thus we need to create intermediate vectors of bindless resources in order
/// to satisfy `wgpu`'s lifetime requirements.
enum BindingResourceArray<'a> {
    /// A list of bindings.
    Buffers(Vec<BufferBinding<'a>>),
    /// A list of texture views.
    TextureViews(Vec<&'a WgpuTextureView>),
    /// A list of samplers.
    Samplers(Vec<&'a WgpuSampler>),
}

/// The location of a material (either bindless or non-bindless) within the
/// slabs.
#[derive(Clone, Copy, Debug, Default, Reflect)]
#[reflect(Clone, Default)]
pub struct MaterialBindingId {
    /// The index of the bind group (slab) where the GPU data is located.
    pub group: MaterialBindGroupIndex,
    /// The slot within that bind group.
    ///
    /// Non-bindless materials will always have a slot of 0.
    pub slot: MaterialBindGroupSlot,
}

/// The index of each material bind group.
///
/// In bindless mode, each bind group contains multiple materials. In
/// non-bindless mode, each bind group contains only one material.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect, Deref, DerefMut)]
#[reflect(Default, Clone, PartialEq, Hash)]
pub struct MaterialBindGroupIndex(pub u32);

impl From<u32> for MaterialBindGroupIndex {
    fn from(value: u32) -> Self {
        MaterialBindGroupIndex(value)
    }
}

/// The index of the slot containing material data within each material bind
/// group.
///
/// In bindless mode, this slot is needed to locate the material data in each
/// bind group, since multiple materials are packed into a single slab. In
/// non-bindless mode, this slot is always 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect, Deref, DerefMut)]
#[reflect(Default, Clone, PartialEq)]
pub struct MaterialBindGroupSlot(pub u32);

/// The CPU/GPU synchronization state of a buffer that we maintain.
///
/// Currently, the only buffer that we maintain is the
/// [`MaterialBindlessIndexTable`].
enum BufferDirtyState {
    /// The buffer is currently synchronized between the CPU and GPU.
    Clean,
    /// The buffer hasn't been created yet.
    NeedsReserve,
    /// The buffer exists on both CPU and GPU, but the GPU data is out of date.
    NeedsUpload,
}

/// Information that describes a potential allocation of an
/// [`UnpreparedBindGroup`] into a slab.
struct BindlessAllocationCandidate {
    /// A map that, for every resource in the [`UnpreparedBindGroup`] that
    /// already existed in this slab, maps bindless index of that resource to
    /// its slot in the appropriate binding array.
    pre_existing_resources: HashMap<BindlessIndex, u32>,
    /// Stores the number of free slots that are needed to satisfy this
    /// allocation.
    needed_free_slots: u32,
}

/// A trait that allows fetching the [`BindingResourceId`] from a
/// [`BindlessResourceType`].
///
/// This is used when freeing bindless resources, in order to locate the IDs
/// assigned to each resource so that they can be removed from the appropriate
/// maps.
trait GetBindingResourceId {
    /// Returns the [`BindingResourceId`] for this resource.
    ///
    /// `resource_type` specifies this resource's type. This is used for
    /// textures, as a `wgpu` [`TextureView`] doesn't store enough information
    /// itself to determine its dimension.
    fn binding_resource_id(&self, resource_type: BindlessResourceType) -> BindingResourceId;
}

/// The public interface to a slab, which represents a single bind group.
pub struct MaterialSlab<'a, M>(MaterialSlabImpl<'a, M>)
where
    M: Material;

/// The actual implementation of a material slab.
///
/// This has bindless and non-bindless variants.
enum MaterialSlabImpl<'a, M>
where
    M: Material,
{
    /// The implementation of the slab interface we use when the slab
    /// is bindless.
    Bindless(&'a MaterialBindlessSlab<M>),
    /// The implementation of the slab interface we use when the slab
    /// is non-bindless.
    NonBindless(MaterialNonBindlessSlab<'a, M>),
}

/// A single bind group that the [`MaterialBindGroupNonBindlessAllocator`]
/// manages.
enum MaterialNonBindlessSlab<'a, M>
where
    M: Material,
{
    /// A slab that has a bind group.
    Prepared(&'a PreparedBindGroup<M::Data>),
    /// A slab that doesn't yet have a bind group.
    Unprepared(&'a UnpreparedBindGroup<M::Data>),
}

/// Manages an array of untyped plain old data on GPU and allocates individual
/// slots within that array.
///
/// This supports the `#[data]` attribute of `AsBindGroup`.
struct MaterialDataBuffer {
    /// The number of the binding that we attach this storage buffer to.
    binding_number: BindingNumber,
    /// The actual data.
    ///
    /// Note that this is untyped (`u8`); the actual aligned size of each
    /// element is given by [`Self::aligned_element_size`];
    buffer: RetainedRawBufferVec<u8>,
    /// The size of each element in the buffer, including padding and alignment
    /// if any.
    aligned_element_size: u32,
    /// A list of free slots within the buffer.
    free_slots: Vec<u32>,
    /// The actual number of slots that have been allocated.
    len: u32,
}

/// A buffer containing plain old data, already packed into the appropriate GPU
/// format, and that can be updated incrementally.
///
/// This structure exists in order to encapsulate the lazy update
/// ([`BufferDirtyState`]) logic in a single place.
#[derive(Deref, DerefMut)]
struct RetainedRawBufferVec<T>
where
    T: Pod,
{
    /// The contents of the buffer.
    #[deref]
    buffer: RawBufferVec<T>,
    /// Whether the contents of the buffer have been uploaded to the GPU.
    dirty: BufferDirtyState,
}

/// The size of the buffer that we assign to unused buffer slots, in bytes.
///
/// This is essentially arbitrary, as it doesn't seem to matter to `wgpu` what
/// the size is.
const DEFAULT_BINDLESS_FALLBACK_BUFFER_SIZE: u64 = 16;

impl From<u32> for MaterialBindGroupSlot {
    fn from(value: u32) -> Self {
        MaterialBindGroupSlot(value)
    }
}

impl From<MaterialBindGroupSlot> for u32 {
    fn from(value: MaterialBindGroupSlot) -> Self {
        value.0
    }
}

impl<'a> From<&'a OwnedBindingResource> for BindingResourceId {
    fn from(value: &'a OwnedBindingResource) -> Self {
        match *value {
            OwnedBindingResource::Buffer(ref buffer) => BindingResourceId::Buffer(buffer.id()),
            OwnedBindingResource::Data(_) => BindingResourceId::DataBuffer,
            OwnedBindingResource::TextureView(ref texture_view_dimension, ref texture_view) => {
                BindingResourceId::TextureView(*texture_view_dimension, texture_view.id())
            }
            OwnedBindingResource::Sampler(_, ref sampler) => {
                BindingResourceId::Sampler(sampler.id())
            }
        }
    }
}

impl GetBindingResourceId for Buffer {
    fn binding_resource_id(&self, _: BindlessResourceType) -> BindingResourceId {
        BindingResourceId::Buffer(self.id())
    }
}

impl GetBindingResourceId for Sampler {
    fn binding_resource_id(&self, _: BindlessResourceType) -> BindingResourceId {
        BindingResourceId::Sampler(self.id())
    }
}

impl GetBindingResourceId for TextureView {
    fn binding_resource_id(&self, resource_type: BindlessResourceType) -> BindingResourceId {
        let texture_view_dimension = match resource_type {
            BindlessResourceType::Texture1d => TextureViewDimension::D1,
            BindlessResourceType::Texture2d => TextureViewDimension::D2,
            BindlessResourceType::Texture2dArray => TextureViewDimension::D2Array,
            BindlessResourceType::Texture3d => TextureViewDimension::D3,
            BindlessResourceType::TextureCube => TextureViewDimension::Cube,
            BindlessResourceType::TextureCubeArray => TextureViewDimension::CubeArray,
            _ => panic!("Resource type is not a texture"),
        };
        BindingResourceId::TextureView(texture_view_dimension, self.id())
    }
}

impl<M> MaterialBindGroupAllocator<M>
where
    M: Material,
{
    /// Creates a new [`MaterialBindGroupAllocator`] managing the data for a
    /// single material.
    fn new(render_device: &RenderDevice) -> MaterialBindGroupAllocator<M> {
        if material_uses_bindless_resources::<M>(render_device) {
            MaterialBindGroupAllocator::Bindless(Box::new(MaterialBindGroupBindlessAllocator::new(
                render_device,
            )))
        } else {
            MaterialBindGroupAllocator::NonBindless(Box::new(
                MaterialBindGroupNonBindlessAllocator::new(),
            ))
        }
    }

    /// Returns the slab with the given index, if one exists.
    pub fn get(&self, group: MaterialBindGroupIndex) -> Option<MaterialSlab<M>> {
        match *self {
            MaterialBindGroupAllocator::Bindless(ref bindless_allocator) => bindless_allocator
                .get(group)
                .map(|bindless_slab| MaterialSlab(MaterialSlabImpl::Bindless(bindless_slab))),
            MaterialBindGroupAllocator::NonBindless(ref non_bindless_allocator) => {
                non_bindless_allocator.get(group).map(|non_bindless_slab| {
                    MaterialSlab(MaterialSlabImpl::NonBindless(non_bindless_slab))
                })
            }
        }
    }

    /// Allocates an [`UnpreparedBindGroup`] and returns the resulting binding ID.
    ///
    /// This method should generally be preferred over
    /// [`Self::allocate_prepared`], because this method supports both bindless
    /// and non-bindless bind groups. Only use [`Self::allocate_prepared`] if
    /// you need to prepare the bind group yourself.
    pub fn allocate_unprepared(
        &mut self,
        unprepared_bind_group: UnpreparedBindGroup<M::Data>,
        bind_group_layout: &BindGroupLayout,
    ) -> MaterialBindingId {
        match *self {
            MaterialBindGroupAllocator::Bindless(
                ref mut material_bind_group_bindless_allocator,
            ) => material_bind_group_bindless_allocator.allocate_unprepared(unprepared_bind_group),
            MaterialBindGroupAllocator::NonBindless(
                ref mut material_bind_group_non_bindless_allocator,
            ) => material_bind_group_non_bindless_allocator
                .allocate_unprepared(unprepared_bind_group, (*bind_group_layout).clone()),
        }
    }

    /// Places a pre-prepared bind group into a slab.
    ///
    /// For bindless materials, the allocator internally manages the bind
    /// groups, so calling this method will panic if this is a bindless
    /// allocator. Only non-bindless allocators support this method.
    ///
    /// It's generally preferred to use [`Self::allocate_unprepared`], because
    /// that method supports both bindless and non-bindless allocators. Only use
    /// this method if you need to prepare the bind group yourself.
    pub fn allocate_prepared(
        &mut self,
        prepared_bind_group: PreparedBindGroup<M::Data>,
    ) -> MaterialBindingId {
        match *self {
            MaterialBindGroupAllocator::Bindless(_) => {
                panic!(
                    "Bindless resources are incompatible with implementing `as_bind_group` \
                     directly; implement `unprepared_bind_group` instead or disable bindless"
                )
            }
            MaterialBindGroupAllocator::NonBindless(ref mut non_bindless_allocator) => {
                non_bindless_allocator.allocate_prepared(prepared_bind_group)
            }
        }
    }

    /// Deallocates the material with the given binding ID.
    ///
    /// Any resources that are no longer referenced are removed from the slab.
    pub fn free(&mut self, material_binding_id: MaterialBindingId) {
        match *self {
            MaterialBindGroupAllocator::Bindless(
                ref mut material_bind_group_bindless_allocator,
            ) => material_bind_group_bindless_allocator.free(material_binding_id),
            MaterialBindGroupAllocator::NonBindless(
                ref mut material_bind_group_non_bindless_allocator,
            ) => material_bind_group_non_bindless_allocator.free(material_binding_id),
        }
    }

    /// Recreates any bind groups corresponding to slabs that have been modified
    /// since last calling [`MaterialBindGroupAllocator::prepare_bind_groups`].
    pub fn prepare_bind_groups(
        &mut self,
        render_device: &RenderDevice,
        fallback_bindless_resources: &FallbackBindlessResources,
        fallback_image: &FallbackImage,
    ) {
        match *self {
            MaterialBindGroupAllocator::Bindless(
                ref mut material_bind_group_bindless_allocator,
            ) => material_bind_group_bindless_allocator.prepare_bind_groups(
                render_device,
                fallback_bindless_resources,
                fallback_image,
            ),
            MaterialBindGroupAllocator::NonBindless(
                ref mut material_bind_group_non_bindless_allocator,
            ) => material_bind_group_non_bindless_allocator.prepare_bind_groups(render_device),
        }
    }

    /// Uploads the contents of all buffers that this
    /// [`MaterialBindGroupAllocator`] manages to the GPU.
    ///
    /// Non-bindless allocators don't currently manage any buffers, so this
    /// method only has an effect for bindless allocators.
    pub fn write_buffers(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        match *self {
            MaterialBindGroupAllocator::Bindless(
                ref mut material_bind_group_bindless_allocator,
            ) => material_bind_group_bindless_allocator.write_buffers(render_device, render_queue),
            MaterialBindGroupAllocator::NonBindless(_) => {
                // Not applicable.
            }
        }
    }
}

impl<M> MaterialBindlessIndexTable<M>
where
    M: Material,
{
    /// Creates a new [`MaterialBindlessIndexTable`] for a single slab.
    fn new(
        bindless_index_table_descriptor: &BindlessIndexTableDescriptor,
    ) -> MaterialBindlessIndexTable<M> {
        // Preallocate space for one bindings table, so that there will always be a buffer.
        let mut buffer = RetainedRawBufferVec::new(BufferUsages::STORAGE);
        for _ in *bindless_index_table_descriptor.indices.start
            ..*bindless_index_table_descriptor.indices.end
        {
            buffer.push(0);
        }

        MaterialBindlessIndexTable {
            buffer,
            index_range: bindless_index_table_descriptor.indices.clone(),
            binding_number: bindless_index_table_descriptor.binding_number,
            phantom: PhantomData,
        }
    }

    /// Returns the bindings in the binding index table.
    ///
    /// If the current [`MaterialBindlessIndexTable::index_range`] is M..N, then
    /// element *i* of the returned binding index table contains the slot of the
    /// bindless resource with bindless index *i* + M.
    fn get(&self, slot: MaterialBindGroupSlot) -> &[u32] {
        let struct_size = *self.index_range.end as usize - *self.index_range.start as usize;
        let start = struct_size * slot.0 as usize;
        &self.buffer.values()[start..(start + struct_size)]
    }

    /// Returns a single binding from the binding index table.
    fn get_binding(
        &self,
        slot: MaterialBindGroupSlot,
        bindless_index: BindlessIndex,
    ) -> Option<u32> {
        if bindless_index < self.index_range.start || bindless_index >= self.index_range.end {
            return None;
        }
        self.get(slot)
            .get((*bindless_index - *self.index_range.start) as usize)
            .copied()
    }

    fn table_length(&self) -> u32 {
        self.index_range.end.0 - self.index_range.start.0
    }

    /// Updates the binding index table for a single material.
    ///
    /// The `allocated_resource_slots` map contains a mapping from the
    /// [`BindlessIndex`] of each resource that the material references to the
    /// slot that that resource occupies in the appropriate binding array. This
    /// method serializes that map into a binding index table that the shader
    /// can read.
    fn set(
        &mut self,
        slot: MaterialBindGroupSlot,
        allocated_resource_slots: &HashMap<BindlessIndex, u32>,
    ) {
        let table_len = self.table_length() as usize;
        let range = (slot.0 as usize * table_len)..((slot.0 as usize + 1) * table_len);
        while self.buffer.len() < range.end {
            self.buffer.push(0);
        }

        for (&bindless_index, &resource_slot) in allocated_resource_slots {
            if self.index_range.contains(&bindless_index) {
                self.buffer.set(
                    *bindless_index + range.start as u32 - *self.index_range.start,
                    resource_slot,
                );
            }
        }

        // Mark the buffer as needing to be recreated, in case we grew it.
        self.buffer.dirty = BufferDirtyState::NeedsReserve;
    }

    /// Returns the [`BindGroupEntry`] for the index table itself.
    fn bind_group_entry(&self) -> BindGroupEntry {
        BindGroupEntry {
            binding: *self.binding_number,
            resource: self
                .buffer
                .buffer()
                .expect("Bindings buffer must exist")
                .as_entire_binding(),
        }
    }
}

impl<T> RetainedRawBufferVec<T>
where
    T: Pod,
{
    /// Creates a new empty [`RetainedRawBufferVec`] supporting the given
    /// [`BufferUsages`].
    fn new(buffer_usages: BufferUsages) -> RetainedRawBufferVec<T> {
        RetainedRawBufferVec {
            buffer: RawBufferVec::new(buffer_usages),
            dirty: BufferDirtyState::NeedsUpload,
        }
    }

    /// Recreates the GPU backing buffer if needed.
    fn prepare(&mut self, render_device: &RenderDevice) {
        match self.dirty {
            BufferDirtyState::Clean | BufferDirtyState::NeedsUpload => {}
            BufferDirtyState::NeedsReserve => {
                let capacity = self.buffer.len();
                self.buffer.reserve(capacity, render_device);
                self.dirty = BufferDirtyState::NeedsUpload;
            }
        }
    }

    /// Writes the current contents of the buffer to the GPU if necessary.
    fn write(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        match self.dirty {
            BufferDirtyState::Clean => {}
            BufferDirtyState::NeedsReserve | BufferDirtyState::NeedsUpload => {
                self.buffer.write_buffer(render_device, render_queue);
                self.dirty = BufferDirtyState::Clean;
            }
        }
    }
}

impl<M> MaterialBindGroupBindlessAllocator<M>
where
    M: Material,
{
    /// Creates a new [`MaterialBindGroupBindlessAllocator`] managing the data
    /// for a single bindless material.
    fn new(render_device: &RenderDevice) -> MaterialBindGroupBindlessAllocator<M> {
        let bindless_descriptor = M::bindless_descriptor()
            .expect("Non-bindless materials should use the non-bindless allocator");
        let fallback_buffers = bindless_descriptor
            .buffers
            .iter()
            .map(|bindless_buffer_descriptor| {
                (
                    bindless_buffer_descriptor.bindless_index,
                    render_device.create_buffer(&BufferDescriptor {
                        label: Some("bindless fallback buffer"),
                        size: match bindless_buffer_descriptor.size {
                            Some(size) => size as u64,
                            None => DEFAULT_BINDLESS_FALLBACK_BUFFER_SIZE,
                        },
                        usage: BufferUsages::STORAGE,
                        mapped_at_creation: false,
                    }),
                )
            })
            .collect();

        MaterialBindGroupBindlessAllocator {
            slabs: vec![],
            bind_group_layout: M::bind_group_layout(render_device),
            bindless_descriptor,
            fallback_buffers,
            slab_capacity: M::bindless_slot_count()
                .expect("Non-bindless materials should use the non-bindless allocator")
                .resolve(),
        }
    }

    /// Allocates the resources for a single material into a slab and returns
    /// the resulting ID.
    ///
    /// The returned [`MaterialBindingId`] can later be used to fetch the slab
    /// that was used.
    ///
    /// This function can't fail. If all slabs are full, then a new slab is
    /// created, and the material is allocated into it.
    fn allocate_unprepared(
        &mut self,
        mut unprepared_bind_group: UnpreparedBindGroup<M::Data>,
    ) -> MaterialBindingId {
        for (slab_index, slab) in self.slabs.iter_mut().enumerate() {
            trace!("Trying to allocate in slab {}", slab_index);
            match slab.try_allocate(unprepared_bind_group, self.slab_capacity) {
                Ok(slot) => {
                    return MaterialBindingId {
                        group: MaterialBindGroupIndex(slab_index as u32),
                        slot,
                    };
                }
                Err(bind_group) => unprepared_bind_group = bind_group,
            }
        }

        let group = MaterialBindGroupIndex(self.slabs.len() as u32);
        self.slabs
            .push(MaterialBindlessSlab::new(&self.bindless_descriptor));

        // Allocate into the newly-pushed slab.
        let Ok(slot) = self
            .slabs
            .last_mut()
            .expect("We just pushed a slab")
            .try_allocate(unprepared_bind_group, self.slab_capacity)
        else {
            panic!("An allocation into an empty slab should always succeed")
        };

        MaterialBindingId { group, slot }
    }

    /// Deallocates the material with the given binding ID.
    ///
    /// Any resources that are no longer referenced are removed from the slab.
    fn free(&mut self, material_binding_id: MaterialBindingId) {
        self.slabs
            .get_mut(material_binding_id.group.0 as usize)
            .expect("Slab should exist")
            .free(material_binding_id.slot, &self.bindless_descriptor);
    }

    /// Returns the slab with the given bind group index.
    ///
    /// A [`MaterialBindGroupIndex`] can be fetched from a
    /// [`MaterialBindingId`].
    fn get(&self, group: MaterialBindGroupIndex) -> Option<&MaterialBindlessSlab<M>> {
        self.slabs.get(group.0 as usize)
    }

    /// Recreates any bind groups corresponding to slabs that have been modified
    /// since last calling
    /// [`MaterialBindGroupBindlessAllocator::prepare_bind_groups`].
    fn prepare_bind_groups(
        &mut self,
        render_device: &RenderDevice,
        fallback_bindless_resources: &FallbackBindlessResources,
        fallback_image: &FallbackImage,
    ) {
        for slab in &mut self.slabs {
            slab.prepare(
                render_device,
                &self.bind_group_layout,
                fallback_bindless_resources,
                &self.fallback_buffers,
                fallback_image,
                &self.bindless_descriptor,
                self.slab_capacity,
            );
        }
    }

    /// Writes any buffers that we're managing to the GPU.
    ///
    /// Currently, this only consists of the bindless index tables.
    fn write_buffers(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        for slab in &mut self.slabs {
            slab.write_buffer(render_device, render_queue);
        }
    }
}

impl<M> FromWorld for MaterialBindGroupAllocator<M>
where
    M: Material,
{
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        MaterialBindGroupAllocator::new(render_device)
    }
}

impl<M> MaterialBindlessSlab<M>
where
    M: Material,
{
    /// Attempts to allocate the given unprepared bind group in this slab.
    ///
    /// If the allocation succeeds, this method returns the slot that the
    /// allocation was placed in. If the allocation fails because the slab was
    /// full, this method returns the unprepared bind group back to the caller
    /// so that it can try to allocate again.
    fn try_allocate(
        &mut self,
        unprepared_bind_group: UnpreparedBindGroup<M::Data>,
        slot_capacity: u32,
    ) -> Result<MaterialBindGroupSlot, UnpreparedBindGroup<M::Data>> {
        // Locate pre-existing resources, and determine how many free slots we need.
        let Some(allocation_candidate) = self.check_allocation(&unprepared_bind_group) else {
            return Err(unprepared_bind_group);
        };

        // Check to see if we have enough free space.
        //
        // As a special case, note that if *nothing* is allocated in this slab,
        // then we always allow a material to be placed in it, regardless of the
        // number of bindings the material has. This is so that, if the
        // platform's maximum bindless count is set too low to hold even a
        // single material, we can still place each material into a separate
        // slab instead of failing outright.
        if self.allocated_resource_count > 0
            && self.allocated_resource_count + allocation_candidate.needed_free_slots
                > slot_capacity
        {
            trace!("Slab is full, can't allocate");
            return Err(unprepared_bind_group);
        }

        // OK, we can allocate in this slab. Assign a slot ID.
        let slot = self
            .free_slots
            .pop()
            .unwrap_or(MaterialBindGroupSlot(self.live_allocation_count));

        // Bump the live allocation count.
        self.live_allocation_count += 1;

        // Insert the resources into the binding arrays.
        let allocated_resource_slots =
            self.insert_resources(unprepared_bind_group.bindings, allocation_candidate);

        // Serialize the allocated resource slots.
        for bindless_index_table in &mut self.bindless_index_tables {
            bindless_index_table.set(slot, &allocated_resource_slots);
        }

        // Insert extra data.
        if self.extra_data.len() < (*slot as usize + 1) {
            self.extra_data.resize_with(*slot as usize + 1, || None);
        }
        self.extra_data[*slot as usize] = Some(unprepared_bind_group.data);

        // Invalidate the cached bind group.
        self.bind_group = None;

        Ok(slot)
    }

    /// Gathers the information needed to determine whether the given unprepared
    /// bind group can be allocated in this slab.
    fn check_allocation(
        &self,
        unprepared_bind_group: &UnpreparedBindGroup<M::Data>,
    ) -> Option<BindlessAllocationCandidate> {
        let mut allocation_candidate = BindlessAllocationCandidate {
            pre_existing_resources: HashMap::default(),
            needed_free_slots: 0,
        };

        for &(bindless_index, ref owned_binding_resource) in unprepared_bind_group.bindings.iter() {
            let bindless_index = BindlessIndex(bindless_index);
            match *owned_binding_resource {
                OwnedBindingResource::Buffer(ref buffer) => {
                    let Some(binding_array) = self.buffers.get(&bindless_index) else {
                        error!(
                            "Binding array wasn't present for buffer at index {:?}",
                            bindless_index
                        );
                        return None;
                    };
                    match binding_array.find(BindingResourceId::Buffer(buffer.id())) {
                        Some(slot) => {
                            allocation_candidate
                                .pre_existing_resources
                                .insert(bindless_index, slot);
                        }
                        None => allocation_candidate.needed_free_slots += 1,
                    }
                }

                OwnedBindingResource::Data(_) => {
                    // The size of a data buffer is unlimited.
                }

                OwnedBindingResource::TextureView(texture_view_dimension, ref texture_view) => {
                    let bindless_resource_type = BindlessResourceType::from(texture_view_dimension);
                    match self
                        .textures
                        .get(&bindless_resource_type)
                        .expect("Missing binding array for texture")
                        .find(BindingResourceId::TextureView(
                            texture_view_dimension,
                            texture_view.id(),
                        )) {
                        Some(slot) => {
                            allocation_candidate
                                .pre_existing_resources
                                .insert(bindless_index, slot);
                        }
                        None => {
                            allocation_candidate.needed_free_slots += 1;
                        }
                    }
                }

                OwnedBindingResource::Sampler(sampler_binding_type, ref sampler) => {
                    let bindless_resource_type = BindlessResourceType::from(sampler_binding_type);
                    match self
                        .samplers
                        .get(&bindless_resource_type)
                        .expect("Missing binding array for sampler")
                        .find(BindingResourceId::Sampler(sampler.id()))
                    {
                        Some(slot) => {
                            allocation_candidate
                                .pre_existing_resources
                                .insert(bindless_index, slot);
                        }
                        None => {
                            allocation_candidate.needed_free_slots += 1;
                        }
                    }
                }
            }
        }

        Some(allocation_candidate)
    }

    /// Inserts the given [`BindingResources`] into this slab.
    ///
    /// Returns a table that maps the bindless index of each resource to its
    /// slot in its binding array.
    fn insert_resources(
        &mut self,
        mut binding_resources: BindingResources,
        allocation_candidate: BindlessAllocationCandidate,
    ) -> HashMap<BindlessIndex, u32> {
        let mut allocated_resource_slots = HashMap::default();

        for (bindless_index, owned_binding_resource) in binding_resources.drain(..) {
            let bindless_index = BindlessIndex(bindless_index);

            let pre_existing_slot = allocation_candidate
                .pre_existing_resources
                .get(&bindless_index);

            // Otherwise, we need to insert it anew.
            let binding_resource_id = BindingResourceId::from(&owned_binding_resource);
            let increment_allocated_resource_count = match owned_binding_resource {
                OwnedBindingResource::Buffer(buffer) => {
                    let slot = self
                        .buffers
                        .get_mut(&bindless_index)
                        .expect("Buffer binding array should exist")
                        .insert(binding_resource_id, buffer);
                    allocated_resource_slots.insert(bindless_index, slot);

                    if let Some(pre_existing_slot) = pre_existing_slot {
                        assert_eq!(*pre_existing_slot, slot);

                        false
                    } else {
                        true
                    }
                }
                OwnedBindingResource::Data(data) => {
                    if pre_existing_slot.is_some() {
                        panic!("Data buffers can't be deduplicated")
                    }

                    let slot = self
                        .data_buffers
                        .get_mut(&bindless_index)
                        .expect("Data buffer binding array should exist")
                        .insert(&data);
                    allocated_resource_slots.insert(bindless_index, slot);
                    false
                }
                OwnedBindingResource::TextureView(texture_view_dimension, texture_view) => {
                    let bindless_resource_type = BindlessResourceType::from(texture_view_dimension);
                    let slot = self
                        .textures
                        .get_mut(&bindless_resource_type)
                        .expect("Texture array should exist")
                        .insert(binding_resource_id, texture_view);
                    allocated_resource_slots.insert(bindless_index, slot);

                    if let Some(pre_existing_slot) = pre_existing_slot {
                        assert_eq!(*pre_existing_slot, slot);

                        false
                    } else {
                        true
                    }
                }
                OwnedBindingResource::Sampler(sampler_binding_type, sampler) => {
                    let bindless_resource_type = BindlessResourceType::from(sampler_binding_type);
                    let slot = self
                        .samplers
                        .get_mut(&bindless_resource_type)
                        .expect("Sampler should exist")
                        .insert(binding_resource_id, sampler);
                    allocated_resource_slots.insert(bindless_index, slot);

                    if let Some(pre_existing_slot) = pre_existing_slot {
                        assert_eq!(*pre_existing_slot, slot);

                        false
                    } else {
                        true
                    }
                }
            };

            // Bump the allocated resource count.
            if increment_allocated_resource_count {
                self.allocated_resource_count += 1;
            }
        }

        allocated_resource_slots
    }

    /// Removes the material allocated in the given slot, with the given
    /// descriptor, from this slab.
    fn free(&mut self, slot: MaterialBindGroupSlot, bindless_descriptor: &BindlessDescriptor) {
        // Loop through each binding.
        for (bindless_index, bindless_resource_type) in
            bindless_descriptor.resources.iter().enumerate()
        {
            let bindless_index = BindlessIndex::from(bindless_index as u32);
            let Some(bindless_index_table) = self.get_bindless_index_table(bindless_index) else {
                continue;
            };
            let Some(bindless_binding) = bindless_index_table.get_binding(slot, bindless_index)
            else {
                continue;
            };

            // Free the binding. If the resource in question was anything other
            // than a data buffer, then it has a reference count and
            // consequently we need to decrement it.
            let decrement_allocated_resource_count = match *bindless_resource_type {
                BindlessResourceType::None => false,
                BindlessResourceType::Buffer => self
                    .buffers
                    .get_mut(&bindless_index)
                    .expect("Buffer should exist with that bindless index")
                    .remove(bindless_binding),
                BindlessResourceType::DataBuffer => {
                    self.data_buffers
                        .get_mut(&bindless_index)
                        .expect("Data buffer should exist with that bindless index")
                        .remove(bindless_binding);
                    false
                }
                BindlessResourceType::SamplerFiltering
                | BindlessResourceType::SamplerNonFiltering
                | BindlessResourceType::SamplerComparison => self
                    .samplers
                    .get_mut(bindless_resource_type)
                    .expect("Sampler array should exist")
                    .remove(bindless_binding),
                BindlessResourceType::Texture1d
                | BindlessResourceType::Texture2d
                | BindlessResourceType::Texture2dArray
                | BindlessResourceType::Texture3d
                | BindlessResourceType::TextureCube
                | BindlessResourceType::TextureCubeArray => self
                    .textures
                    .get_mut(bindless_resource_type)
                    .expect("Texture array should exist")
                    .remove(bindless_binding),
            };

            // If the slot is now free, decrement the allocated resource
            // count.
            if decrement_allocated_resource_count {
                self.allocated_resource_count -= 1;
            }
        }

        // Clear out the extra data.
        self.extra_data[slot.0 as usize] = None;

        // Invalidate the cached bind group.
        self.bind_group = None;

        // Release the slot ID.
        self.free_slots.push(slot);
        self.live_allocation_count -= 1;
    }

    /// Recreates the bind group and bindless index table buffer if necessary.
    fn prepare(
        &mut self,
        render_device: &RenderDevice,
        bind_group_layout: &BindGroupLayout,
        fallback_bindless_resources: &FallbackBindlessResources,
        fallback_buffers: &HashMap<BindlessIndex, Buffer>,
        fallback_image: &FallbackImage,
        bindless_descriptor: &BindlessDescriptor,
        slab_capacity: u32,
    ) {
        // Create the bindless index table buffers if needed.
        for bindless_index_table in &mut self.bindless_index_tables {
            bindless_index_table.buffer.prepare(render_device);
        }

        // Create any data buffers we were managing if necessary.
        for data_buffer in self.data_buffers.values_mut() {
            data_buffer.buffer.prepare(render_device);
        }

        // Create the bind group if needed.
        self.prepare_bind_group(
            render_device,
            bind_group_layout,
            fallback_bindless_resources,
            fallback_buffers,
            fallback_image,
            bindless_descriptor,
            slab_capacity,
        );
    }

    /// Recreates the bind group if this slab has been changed since the last
    /// time we created it.
    fn prepare_bind_group(
        &mut self,
        render_device: &RenderDevice,
        bind_group_layout: &BindGroupLayout,
        fallback_bindless_resources: &FallbackBindlessResources,
        fallback_buffers: &HashMap<BindlessIndex, Buffer>,
        fallback_image: &FallbackImage,
        bindless_descriptor: &BindlessDescriptor,
        slab_capacity: u32,
    ) {
        // If the bind group is clean, then do nothing.
        if self.bind_group.is_some() {
            return;
        }

        // Determine whether we need to pad out our binding arrays with dummy
        // resources.
        let required_binding_array_size = if render_device
            .features()
            .contains(WgpuFeatures::PARTIALLY_BOUND_BINDING_ARRAY)
        {
            None
        } else {
            Some(slab_capacity)
        };

        let binding_resource_arrays = self.create_binding_resource_arrays(
            fallback_bindless_resources,
            fallback_buffers,
            fallback_image,
            bindless_descriptor,
            required_binding_array_size,
        );

        let mut bind_group_entries: Vec<_> = self
            .bindless_index_tables
            .iter()
            .map(|bindless_index_table| bindless_index_table.bind_group_entry())
            .collect();

        for &(&binding, ref binding_resource_array) in binding_resource_arrays.iter() {
            bind_group_entries.push(BindGroupEntry {
                binding,
                resource: match *binding_resource_array {
                    BindingResourceArray::Buffers(ref buffer_bindings) => {
                        BindingResource::BufferArray(&buffer_bindings[..])
                    }
                    BindingResourceArray::TextureViews(ref texture_views) => {
                        BindingResource::TextureViewArray(&texture_views[..])
                    }
                    BindingResourceArray::Samplers(ref samplers) => {
                        BindingResource::SamplerArray(&samplers[..])
                    }
                },
            });
        }

        // Create bind group entries for any data buffers we're managing.
        for data_buffer in self.data_buffers.values() {
            bind_group_entries.push(BindGroupEntry {
                binding: *data_buffer.binding_number,
                resource: data_buffer
                    .buffer
                    .buffer()
                    .expect("Backing data buffer must have been uploaded by now")
                    .as_entire_binding(),
            });
        }

        self.bind_group = Some(render_device.create_bind_group(
            M::label(),
            bind_group_layout,
            &bind_group_entries,
        ));
    }

    /// Writes any buffers that we're managing to the GPU.
    ///
    /// Currently, this consists of the bindless index table plus any data
    /// buffers we're managing.
    fn write_buffer(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        for bindless_index_table in &mut self.bindless_index_tables {
            bindless_index_table
                .buffer
                .write(render_device, render_queue);
        }

        for data_buffer in self.data_buffers.values_mut() {
            data_buffer.buffer.write(render_device, render_queue);
        }
    }

    /// Converts our binding arrays into binding resource arrays suitable for
    /// passing to `wgpu`.
    fn create_binding_resource_arrays<'a>(
        &'a self,
        fallback_bindless_resources: &'a FallbackBindlessResources,
        fallback_buffers: &'a HashMap<BindlessIndex, Buffer>,
        fallback_image: &'a FallbackImage,
        bindless_descriptor: &'a BindlessDescriptor,
        required_binding_array_size: Option<u32>,
    ) -> Vec<(&'a u32, BindingResourceArray<'a>)> {
        let mut binding_resource_arrays = vec![];

        // Build sampler bindings.
        self.create_sampler_binding_resource_arrays(
            &mut binding_resource_arrays,
            fallback_bindless_resources,
            required_binding_array_size,
        );

        // Build texture bindings.
        self.create_texture_binding_resource_arrays(
            &mut binding_resource_arrays,
            fallback_image,
            required_binding_array_size,
        );

        // Build buffer bindings.
        self.create_buffer_binding_resource_arrays(
            &mut binding_resource_arrays,
            fallback_buffers,
            bindless_descriptor,
            required_binding_array_size,
        );

        binding_resource_arrays
    }

    /// Accumulates sampler binding arrays into binding resource arrays suitable
    /// for passing to `wgpu`.
    fn create_sampler_binding_resource_arrays<'a, 'b>(
        &'a self,
        binding_resource_arrays: &'b mut Vec<(&'a u32, BindingResourceArray<'a>)>,
        fallback_bindless_resources: &'a FallbackBindlessResources,
        required_binding_array_size: Option<u32>,
    ) {
        // We have one binding resource array per sampler type.
        for (bindless_resource_type, fallback_sampler) in [
            (
                BindlessResourceType::SamplerFiltering,
                &fallback_bindless_resources.filtering_sampler,
            ),
            (
                BindlessResourceType::SamplerNonFiltering,
                &fallback_bindless_resources.non_filtering_sampler,
            ),
            (
                BindlessResourceType::SamplerComparison,
                &fallback_bindless_resources.comparison_sampler,
            ),
        ] {
            let mut sampler_bindings = vec![];

            match self.samplers.get(&bindless_resource_type) {
                Some(sampler_bindless_binding_array) => {
                    for maybe_bindless_binding in sampler_bindless_binding_array.bindings.iter() {
                        match *maybe_bindless_binding {
                            Some(ref bindless_binding) => {
                                sampler_bindings.push(&*bindless_binding.resource);
                            }
                            None => sampler_bindings.push(&**fallback_sampler),
                        }
                    }
                }

                None => {
                    // Fill with a single fallback sampler.
                    sampler_bindings.push(&**fallback_sampler);
                }
            }

            if let Some(required_binding_array_size) = required_binding_array_size {
                sampler_bindings.extend(iter::repeat_n(
                    &**fallback_sampler,
                    required_binding_array_size as usize - sampler_bindings.len(),
                ));
            }

            let binding_number = bindless_resource_type
                .binding_number()
                .expect("Sampler bindless resource type must have a binding number");

            binding_resource_arrays.push((
                &**binding_number,
                BindingResourceArray::Samplers(sampler_bindings),
            ));
        }
    }

    /// Accumulates texture binding arrays into binding resource arrays suitable
    /// for passing to `wgpu`.
    fn create_texture_binding_resource_arrays<'a, 'b>(
        &'a self,
        binding_resource_arrays: &'b mut Vec<(&'a u32, BindingResourceArray<'a>)>,
        fallback_image: &'a FallbackImage,
        required_binding_array_size: Option<u32>,
    ) {
        for (bindless_resource_type, fallback_image) in [
            (BindlessResourceType::Texture1d, &fallback_image.d1),
            (BindlessResourceType::Texture2d, &fallback_image.d2),
            (
                BindlessResourceType::Texture2dArray,
                &fallback_image.d2_array,
            ),
            (BindlessResourceType::Texture3d, &fallback_image.d3),
            (BindlessResourceType::TextureCube, &fallback_image.cube),
            (
                BindlessResourceType::TextureCubeArray,
                &fallback_image.cube_array,
            ),
        ] {
            let mut texture_bindings = vec![];

            let binding_number = bindless_resource_type
                .binding_number()
                .expect("Texture bindless resource type must have a binding number");

            match self.textures.get(&bindless_resource_type) {
                Some(texture_bindless_binding_array) => {
                    for maybe_bindless_binding in texture_bindless_binding_array.bindings.iter() {
                        match *maybe_bindless_binding {
                            Some(ref bindless_binding) => {
                                texture_bindings.push(&*bindless_binding.resource);
                            }
                            None => texture_bindings.push(&*fallback_image.texture_view),
                        }
                    }
                }

                None => {
                    // Fill with a single fallback image.
                    texture_bindings.push(&*fallback_image.texture_view);
                }
            }

            if let Some(required_binding_array_size) = required_binding_array_size {
                texture_bindings.extend(iter::repeat_n(
                    &*fallback_image.texture_view,
                    required_binding_array_size as usize - texture_bindings.len(),
                ));
            }

            binding_resource_arrays.push((
                binding_number,
                BindingResourceArray::TextureViews(texture_bindings),
            ));
        }
    }

    /// Accumulates buffer binding arrays into binding resource arrays suitable
    /// for `wgpu`.
    fn create_buffer_binding_resource_arrays<'a, 'b>(
        &'a self,
        binding_resource_arrays: &'b mut Vec<(&'a u32, BindingResourceArray<'a>)>,
        fallback_buffers: &'a HashMap<BindlessIndex, Buffer>,
        bindless_descriptor: &'a BindlessDescriptor,
        required_binding_array_size: Option<u32>,
    ) {
        for bindless_buffer_descriptor in bindless_descriptor.buffers.iter() {
            let Some(buffer_bindless_binding_array) =
                self.buffers.get(&bindless_buffer_descriptor.bindless_index)
            else {
                // This is OK, because index buffers are present in
                // `BindlessDescriptor::buffers` but not in
                // `BindlessDescriptor::resources`.
                continue;
            };

            let fallback_buffer = fallback_buffers
                .get(&bindless_buffer_descriptor.bindless_index)
                .expect("Fallback buffer should exist");

            let mut buffer_bindings: Vec<_> = buffer_bindless_binding_array
                .bindings
                .iter()
                .map(|maybe_bindless_binding| {
                    let buffer = match *maybe_bindless_binding {
                        None => fallback_buffer,
                        Some(ref bindless_binding) => &bindless_binding.resource,
                    };
                    BufferBinding {
                        buffer,
                        offset: 0,
                        size: None,
                    }
                })
                .collect();

            if let Some(required_binding_array_size) = required_binding_array_size {
                buffer_bindings.extend(iter::repeat_n(
                    BufferBinding {
                        buffer: fallback_buffer,
                        offset: 0,
                        size: None,
                    },
                    required_binding_array_size as usize - buffer_bindings.len(),
                ));
            }

            binding_resource_arrays.push((
                &*buffer_bindless_binding_array.binding_number,
                BindingResourceArray::Buffers(buffer_bindings),
            ));
        }
    }

    /// Returns the [`BindGroup`] corresponding to this slab, if it's been
    /// prepared.
    fn bind_group(&self) -> Option<&BindGroup> {
        self.bind_group.as_ref()
    }

    /// Returns the extra data associated with this material.
    fn get_extra_data(&self, slot: MaterialBindGroupSlot) -> &M::Data {
        self.extra_data
            .get(slot.0 as usize)
            .and_then(|data| data.as_ref())
            .expect("Extra data not present")
    }

    /// Returns the bindless index table containing the given bindless index.
    fn get_bindless_index_table(
        &self,
        bindless_index: BindlessIndex,
    ) -> Option<&MaterialBindlessIndexTable<M>> {
        let table_index = self
            .bindless_index_tables
            .binary_search_by(|bindless_index_table| {
                if bindless_index < bindless_index_table.index_range.start {
                    Ordering::Less
                } else if bindless_index >= bindless_index_table.index_range.end {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            })
            .ok()?;
        self.bindless_index_tables.get(table_index)
    }
}

impl<R> MaterialBindlessBindingArray<R>
where
    R: GetBindingResourceId,
{
    /// Creates a new [`MaterialBindlessBindingArray`] with the given binding
    /// number, managing resources of the given type.
    fn new(
        binding_number: BindingNumber,
        resource_type: BindlessResourceType,
    ) -> MaterialBindlessBindingArray<R> {
        MaterialBindlessBindingArray {
            binding_number,
            bindings: vec![],
            resource_type,
            resource_to_slot: HashMap::default(),
            free_slots: vec![],
            len: 0,
        }
    }

    /// Returns the slot corresponding to the given resource, if that resource
    /// is located in this binding array.
    ///
    /// If the resource isn't in this binding array, this method returns `None`.
    fn find(&self, binding_resource_id: BindingResourceId) -> Option<u32> {
        self.resource_to_slot.get(&binding_resource_id).copied()
    }

    /// Inserts a bindless resource into a binding array and returns the index
    /// of the slot it was inserted into.
    fn insert(&mut self, binding_resource_id: BindingResourceId, resource: R) -> u32 {
        match self.resource_to_slot.entry(binding_resource_id) {
            bevy_platform::collections::hash_map::Entry::Occupied(o) => {
                let slot = *o.get();

                self.bindings[slot as usize]
                    .as_mut()
                    .expect("A slot in the resource_to_slot map should have a value")
                    .ref_count += 1;

                slot
            }
            bevy_platform::collections::hash_map::Entry::Vacant(v) => {
                let slot = self.free_slots.pop().unwrap_or(self.len);
                v.insert(slot);

                if self.bindings.len() < slot as usize + 1 {
                    self.bindings.resize_with(slot as usize + 1, || None);
                }
                self.bindings[slot as usize] = Some(MaterialBindlessBinding::new(resource));

                self.len += 1;
                slot
            }
        }
    }

    /// Removes a reference to an object from the slot.
    ///
    /// If the reference count dropped to 0 and the object was freed, this
    /// method returns true. If the object was still referenced after removing
    /// it, returns false.
    fn remove(&mut self, slot: u32) -> bool {
        let maybe_binding = &mut self.bindings[slot as usize];
        let binding = maybe_binding
            .as_mut()
            .expect("Attempted to free an already-freed binding");

        binding.ref_count -= 1;
        if binding.ref_count != 0 {
            return false;
        }

        let binding_resource_id = binding.resource.binding_resource_id(self.resource_type);
        self.resource_to_slot.remove(&binding_resource_id);

        *maybe_binding = None;
        self.free_slots.push(slot);
        self.len -= 1;
        true
    }
}

impl<R> MaterialBindlessBinding<R>
where
    R: GetBindingResourceId,
{
    /// Creates a new [`MaterialBindlessBinding`] for a freshly-added resource.
    ///
    /// The reference count is initialized to 1.
    fn new(resource: R) -> MaterialBindlessBinding<R> {
        MaterialBindlessBinding {
            resource,
            ref_count: 1,
        }
    }
}

/// Returns true if the material will *actually* use bindless resources or false
/// if it won't.
///
/// This takes the platform support (or lack thereof) for bindless resources
/// into account.
pub fn material_uses_bindless_resources<M>(render_device: &RenderDevice) -> bool
where
    M: Material,
{
    M::bindless_slot_count().is_some_and(|bindless_slot_count| {
        M::bindless_supported(render_device) && bindless_slot_count.resolve() > 1
    })
}

impl<M> MaterialBindlessSlab<M>
where
    M: Material,
{
    /// Creates a new [`MaterialBindlessSlab`] for a material with the given
    /// bindless descriptor.
    ///
    /// We use this when no existing slab could hold a material to be allocated.
    fn new(bindless_descriptor: &BindlessDescriptor) -> MaterialBindlessSlab<M> {
        let mut buffers = HashMap::default();
        let mut samplers = HashMap::default();
        let mut textures = HashMap::default();
        let mut data_buffers = HashMap::default();

        for (bindless_index, bindless_resource_type) in
            bindless_descriptor.resources.iter().enumerate()
        {
            let bindless_index = BindlessIndex(bindless_index as u32);
            match *bindless_resource_type {
                BindlessResourceType::None => {}
                BindlessResourceType::Buffer => {
                    let binding_number = bindless_descriptor
                        .buffers
                        .iter()
                        .find(|bindless_buffer_descriptor| {
                            bindless_buffer_descriptor.bindless_index == bindless_index
                        })
                        .expect(
                            "Bindless buffer descriptor matching that bindless index should be \
                             present",
                        )
                        .binding_number;
                    buffers.insert(
                        bindless_index,
                        MaterialBindlessBindingArray::new(binding_number, *bindless_resource_type),
                    );
                }
                BindlessResourceType::DataBuffer => {
                    // Copy the data in.
                    let buffer_descriptor = bindless_descriptor
                        .buffers
                        .iter()
                        .find(|bindless_buffer_descriptor| {
                            bindless_buffer_descriptor.bindless_index == bindless_index
                        })
                        .expect(
                            "Bindless buffer descriptor matching that bindless index should be \
                             present",
                        );
                    data_buffers.insert(
                        bindless_index,
                        MaterialDataBuffer::new(
                            buffer_descriptor.binding_number,
                            buffer_descriptor
                                .size
                                .expect("Data buffers should have a size")
                                as u32,
                        ),
                    );
                }
                BindlessResourceType::SamplerFiltering
                | BindlessResourceType::SamplerNonFiltering
                | BindlessResourceType::SamplerComparison => {
                    samplers.insert(
                        *bindless_resource_type,
                        MaterialBindlessBindingArray::new(
                            *bindless_resource_type.binding_number().unwrap(),
                            *bindless_resource_type,
                        ),
                    );
                }
                BindlessResourceType::Texture1d
                | BindlessResourceType::Texture2d
                | BindlessResourceType::Texture2dArray
                | BindlessResourceType::Texture3d
                | BindlessResourceType::TextureCube
                | BindlessResourceType::TextureCubeArray => {
                    textures.insert(
                        *bindless_resource_type,
                        MaterialBindlessBindingArray::new(
                            *bindless_resource_type.binding_number().unwrap(),
                            *bindless_resource_type,
                        ),
                    );
                }
            }
        }

        let bindless_index_tables = bindless_descriptor
            .index_tables
            .iter()
            .map(|bindless_index_table| MaterialBindlessIndexTable::new(bindless_index_table))
            .collect();

        MaterialBindlessSlab {
            bind_group: None,
            bindless_index_tables,
            samplers,
            textures,
            buffers,
            data_buffers,
            extra_data: vec![],
            free_slots: vec![],
            live_allocation_count: 0,
            allocated_resource_count: 0,
        }
    }
}

impl FromWorld for FallbackBindlessResources {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        FallbackBindlessResources {
            filtering_sampler: render_device.create_sampler(&SamplerDescriptor {
                label: Some("fallback filtering sampler"),
                ..default()
            }),
            non_filtering_sampler: render_device.create_sampler(&SamplerDescriptor {
                label: Some("fallback non-filtering sampler"),
                mag_filter: FilterMode::Nearest,
                min_filter: FilterMode::Nearest,
                mipmap_filter: FilterMode::Nearest,
                ..default()
            }),
            comparison_sampler: render_device.create_sampler(&SamplerDescriptor {
                label: Some("fallback comparison sampler"),
                compare: Some(CompareFunction::Always),
                ..default()
            }),
        }
    }
}

impl<M> MaterialBindGroupNonBindlessAllocator<M>
where
    M: Material,
{
    /// Creates a new [`MaterialBindGroupNonBindlessAllocator`] managing the
    /// bind groups for a single non-bindless material.
    fn new() -> MaterialBindGroupNonBindlessAllocator<M> {
        MaterialBindGroupNonBindlessAllocator {
            bind_groups: vec![],
            to_prepare: HashSet::default(),
            free_indices: vec![],
            phantom: PhantomData,
        }
    }

    /// Inserts a bind group, either unprepared or prepared, into this allocator
    /// and returns a [`MaterialBindingId`].
    ///
    /// The returned [`MaterialBindingId`] can later be used to fetch the bind
    /// group.
    fn allocate(
        &mut self,
        bind_group: MaterialNonBindlessAllocatedBindGroup<M>,
    ) -> MaterialBindingId {
        let group_id = self
            .free_indices
            .pop()
            .unwrap_or(MaterialBindGroupIndex(self.bind_groups.len() as u32));
        if self.bind_groups.len() < *group_id as usize + 1 {
            self.bind_groups
                .resize_with(*group_id as usize + 1, || None);
        }

        if matches!(
            bind_group,
            MaterialNonBindlessAllocatedBindGroup::Unprepared { .. }
        ) {
            self.to_prepare.insert(group_id);
        }

        self.bind_groups[*group_id as usize] = Some(bind_group);

        MaterialBindingId {
            group: group_id,
            slot: default(),
        }
    }

    /// Inserts an unprepared bind group into this allocator and returns a
    /// [`MaterialBindingId`].
    fn allocate_unprepared(
        &mut self,
        unprepared_bind_group: UnpreparedBindGroup<M::Data>,
        bind_group_layout: BindGroupLayout,
    ) -> MaterialBindingId {
        self.allocate(MaterialNonBindlessAllocatedBindGroup::Unprepared {
            bind_group: unprepared_bind_group,
            layout: bind_group_layout,
        })
    }

    /// Inserts an prepared bind group into this allocator and returns a
    /// [`MaterialBindingId`].
    fn allocate_prepared(
        &mut self,
        prepared_bind_group: PreparedBindGroup<M::Data>,
    ) -> MaterialBindingId {
        self.allocate(MaterialNonBindlessAllocatedBindGroup::Prepared {
            bind_group: prepared_bind_group,
            uniform_buffers: vec![],
        })
    }

    /// Deallocates the bind group with the given binding ID.
    fn free(&mut self, binding_id: MaterialBindingId) {
        debug_assert_eq!(binding_id.slot, MaterialBindGroupSlot(0));
        debug_assert!(self.bind_groups[*binding_id.group as usize].is_some());
        self.bind_groups[*binding_id.group as usize] = None;
        self.to_prepare.remove(&binding_id.group);
        self.free_indices.push(binding_id.group);
    }

    /// Returns a wrapper around the bind group with the given index.
    fn get(&self, group: MaterialBindGroupIndex) -> Option<MaterialNonBindlessSlab<M>> {
        self.bind_groups[group.0 as usize]
            .as_ref()
            .map(|bind_group| match bind_group {
                MaterialNonBindlessAllocatedBindGroup::Prepared { bind_group, .. } => {
                    MaterialNonBindlessSlab::Prepared(bind_group)
                }
                MaterialNonBindlessAllocatedBindGroup::Unprepared { bind_group, .. } => {
                    MaterialNonBindlessSlab::Unprepared(bind_group)
                }
            })
    }

    /// Prepares any as-yet unprepared bind groups that this allocator is
    /// managing.
    ///
    /// Unprepared bind groups can be added to this allocator with
    /// [`Self::allocate_unprepared`]. Such bind groups will defer being
    /// prepared until the next time this method is called.
    fn prepare_bind_groups(&mut self, render_device: &RenderDevice) {
        for bind_group_index in mem::take(&mut self.to_prepare) {
            let Some(MaterialNonBindlessAllocatedBindGroup::Unprepared {
                bind_group: unprepared_bind_group,
                layout: bind_group_layout,
            }) = mem::take(&mut self.bind_groups[*bind_group_index as usize])
            else {
                panic!("Allocation didn't exist or was already prepared");
            };

            // Pack any `Data` into uniform buffers.
            let mut uniform_buffers = vec![];
            for (index, binding) in unprepared_bind_group.bindings.iter() {
                let OwnedBindingResource::Data(ref owned_data) = *binding else {
                    continue;
                };
                let label = format!("material uniform data {}", *index);
                let uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some(&label),
                    contents: &owned_data.0,
                    usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
                });
                uniform_buffers.push(uniform_buffer);
            }

            // Create bind group entries.
            let mut bind_group_entries = vec![];
            let mut uniform_buffers_iter = uniform_buffers.iter();
            for (index, binding) in unprepared_bind_group.bindings.iter() {
                match *binding {
                    OwnedBindingResource::Data(_) => {
                        bind_group_entries.push(BindGroupEntry {
                            binding: *index,
                            resource: uniform_buffers_iter
                                .next()
                                .expect("We should have created uniform buffers for each `Data`")
                                .as_entire_binding(),
                        });
                    }
                    _ => bind_group_entries.push(BindGroupEntry {
                        binding: *index,
                        resource: binding.get_binding(),
                    }),
                }
            }

            // Create the bind group.
            let bind_group = render_device.create_bind_group(
                M::label(),
                &bind_group_layout,
                &bind_group_entries,
            );

            self.bind_groups[*bind_group_index as usize] =
                Some(MaterialNonBindlessAllocatedBindGroup::Prepared {
                    bind_group: PreparedBindGroup {
                        bindings: unprepared_bind_group.bindings,
                        bind_group,
                        data: unprepared_bind_group.data,
                    },
                    uniform_buffers,
                });
        }
    }
}

impl<'a, M> MaterialSlab<'a, M>
where
    M: Material,
{
    /// Returns the extra data associated with this material.
    ///
    /// When deriving `AsBindGroup`, this data is given by the
    /// `#[bind_group_data(DataType)]` attribute on the material structure.
    pub fn get_extra_data(&self, slot: MaterialBindGroupSlot) -> &M::Data {
        match self.0 {
            MaterialSlabImpl::Bindless(material_bindless_slab) => {
                material_bindless_slab.get_extra_data(slot)
            }
            MaterialSlabImpl::NonBindless(MaterialNonBindlessSlab::Prepared(
                prepared_bind_group,
            )) => &prepared_bind_group.data,
            MaterialSlabImpl::NonBindless(MaterialNonBindlessSlab::Unprepared(
                unprepared_bind_group,
            )) => &unprepared_bind_group.data,
        }
    }

    /// Returns the [`BindGroup`] corresponding to this slab, if it's been
    /// prepared.
    ///
    /// You can prepare bind groups by calling
    /// [`MaterialBindGroupAllocator::prepare_bind_groups`]. If the bind group
    /// isn't ready, this method returns `None`.
    pub fn bind_group(&self) -> Option<&'a BindGroup> {
        match self.0 {
            MaterialSlabImpl::Bindless(material_bindless_slab) => {
                material_bindless_slab.bind_group()
            }
            MaterialSlabImpl::NonBindless(MaterialNonBindlessSlab::Prepared(
                prepared_bind_group,
            )) => Some(&prepared_bind_group.bind_group),
            MaterialSlabImpl::NonBindless(MaterialNonBindlessSlab::Unprepared(_)) => None,
        }
    }
}

impl MaterialDataBuffer {
    /// Creates a new [`MaterialDataBuffer`] managing a buffer of elements of
    /// size `aligned_element_size` that will be bound to the given binding
    /// number.
    fn new(binding_number: BindingNumber, aligned_element_size: u32) -> MaterialDataBuffer {
        MaterialDataBuffer {
            binding_number,
            buffer: RetainedRawBufferVec::new(BufferUsages::STORAGE),
            aligned_element_size,
            free_slots: vec![],
            len: 0,
        }
    }

    /// Allocates a slot for a new piece of data, copies the data into that
    /// slot, and returns the slot ID.
    ///
    /// The size of the piece of data supplied to this method must equal the
    /// [`Self::aligned_element_size`] provided to [`MaterialDataBuffer::new`].
    fn insert(&mut self, data: &[u8]) -> u32 {
        // Make the the data is of the right length.
        debug_assert_eq!(data.len(), self.aligned_element_size as usize);

        // Grab a slot.
        let slot = self.free_slots.pop().unwrap_or(self.len);

        // Calculate the range we're going to copy to.
        let start = slot as usize * self.aligned_element_size as usize;
        let end = (slot as usize + 1) * self.aligned_element_size as usize;

        // Resize the buffer if necessary.
        if self.buffer.len() < end {
            self.buffer.reserve_internal(end);
        }
        while self.buffer.values().len() < end {
            self.buffer.push(0);
        }

        // Copy in the data.
        self.buffer.values_mut()[start..end].copy_from_slice(data);

        // Mark the buffer dirty, and finish up.
        self.len += 1;
        self.buffer.dirty = BufferDirtyState::NeedsReserve;
        slot
    }

    /// Marks the given slot as free.
    fn remove(&mut self, slot: u32) {
        self.free_slots.push(slot);
        self.len -= 1;
    }
}
