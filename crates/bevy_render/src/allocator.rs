//! An allocator that divides up vertex and index buffers so that multiple
//! meshes can be packed together.
//!
//! The underlying allocation algorithm is [`offset-allocator`], which is a port
//! of [Sebastian Aaltonen's `OffsetAllocator`]. It's a fast, simple hard real
//! time allocator in the two-level segregated fit family.
//!
//! Allocations are divided into two categories: *regular* and *large*. Regular
//! allocations go into one of the shared slabs managed by an allocator. Large
//! allocations get their own individual slabs. Due to platform limitations, on
//! WebGL 2 all vertex buffers are considered large allocations that get their
//! own slabs.
//!
//! The purpose of packing meshes together is to reduce the number of times
//! vertex and index buffers have to be re-bound, which is expensive.
//!
//! [`offset-allocator`]: https://github.com/pcwalton/offset-allocator/
//! [Sebastian Aaltonen's `OffsetAllocator`]: https://github.com/sebbbi/OffsetAllocator

use std::{
    fmt::{self, Debug, Display, Formatter},
    hash::{DefaultHasher, Hash, Hasher},
    iter,
    sync::{Arc, RwLock},
};

use bevy_app::{App, Plugin};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    schedule::IntoSystemConfigs,
    system::{ResMut, Resource},
};
use bevy_time::common_conditions::on_timer;
use bevy_utils::{hashbrown::HashMap, prelude::default, tracing::error, Duration};
use offset_allocator::{Allocation, Allocator};
use slotmap::{new_key_type, SlotMap};
use wgpu::{
    util::BufferInitDescriptor, BufferDescriptor, BufferUsages, DownlevelFlags, IndexFormat,
};

use crate::{
    mesh::MeshVertexBufferLayoutRef,
    render_resource::Buffer,
    renderer::{RenderAdapter, RenderDevice, RenderQueue},
    Render, RenderApp,
};

/// How often we sweep unused allocations.
const SWEEP_INTERVAL: Duration = Duration::from_secs(10);

/// The default size of a slab, in bytes.
const DEFAULT_SLAB_SIZE: u64 = 32 * 1024 * 1024;

/// A plugin that provides the GPU memory allocator.
pub struct GpuAllocatorPlugin {
    /// The size of a slab.
    ///
    /// By default, this is 32 MB.
    pub slab_size: u64,
}

/// Manages allocations in GPU buffers.
#[derive(Resource, Clone)]
pub struct GpuAllocator {
    slabs: HashMap<SlabId, Buffer>,
    slab_size: u64,
    next_slab_id: SlabId,
    classes: HashMap<GpuAllocationClass, GpuClassAllocator>,
    adapter_downlevel_flags: DownlevelFlags,
}

#[derive(Clone, Default, Deref, DerefMut)]
struct GpuClassAllocator(Arc<RwLock<GpuClassAllocatorData>>);

#[derive(Default)]
struct GpuClassAllocatorData {
    regular_slabs: SlotMap<RegularSlabId, (Allocator, SlabId)>,
    large_slabs: SlotMap<LargeSlabId, SlabId>,
    free_large_slabs: Vec<SlabId>,
}

/// The type of a GPU buffer. Each class has its own allocator.
///
/// Only allocations of the same class can coexist in a single buffer.
///
/// Unlike regular CPU memory, GPU buffers require allocation with specific
/// *usages*, which restrict how they can be used. Additionally, the APIs place
/// additional restrictions: for example, because drawcalls require us to
/// specify the initial vertex *index*, and not the initial vertex *byte
/// position*, we must only group meshes with identical vertex buffer layouts
/// into the same buffer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum GpuAllocationClass {
    /// A buffer for holding mesh vertex data conforming to the given layout.
    VertexBuffer(MeshVertexBufferLayoutRef),
    /// A buffer for holding mesh index data, with the given data type.
    IndexBuffer(IndexFormat),
}

/// Identifies a single buffer.
///
/// We don't use a [`SlotMap`] for these because we want
/// monotonically-increasing integers to achieve a consistent distribution range
/// in the [`crate::mesh::MeshSlabHash`]. If we used a [`SlotMap`], then we
/// could have unpredictable collisions, resulting in harder-to-diagnose
/// performance issues.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Deref, DerefMut, Debug)]
#[repr(transparent)]
pub struct SlabId(pub u32);

new_key_type! {
    /// The index of a regular slab in the
    /// `GpuClassAllocatorData::regular_slabs` index.
    ///
    /// Note that this is distinct from a [`SlabId`].
    pub struct RegularSlabId;
}

new_key_type! {
    /// The index of a large slab in the `GpuClassAllocatorData::large_slabs`
    /// index.
    ///
    /// Note that this is distinct from a [`SlabId`].
    pub struct LargeSlabId;
}

/// A handle to an allocation.
///
/// This information can be used to look up the buffer and offset.
///
/// When this handle is dropped, the allocation is automatically freed. This
/// type isn't clonable; if you want to hand out multiple references, wrap it in
/// an [`Arc`].
pub struct GpuAllocation {
    /// The ID of the allocation in the allocation tables.
    allocation_id: GpuAllocationId,

    /// The ID of the buffer in which this allocation lives.
    ///
    /// This could be fetched from the allocator, but caching it here is faster.
    slab_id: SlabId,

    /// This is the offset in `unit_size` elements. It may differ from the
    /// offset in `Allocation` because the one in `Allocation` is in multiples
    /// of `aligned_unit_size`, while this one is in multiples of `unit_size`.
    offset: u32,

    /// A handle to the allocation class that this comes from.
    class_allocator: GpuClassAllocator,
}

/// Identifies an allocation in the allocation tables.
#[derive(Clone)]
enum GpuAllocationId {
    /// This allocation is potentially grouped with others as part of a slab.
    Regular(RegularSlabId, Allocation),
    /// This allocation has its own slab.
    Large(LargeSlabId),
}

impl Plugin for GpuAllocatorPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(Render, free_unused_slabs.run_if(on_timer(SWEEP_INTERVAL)));
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        let render_adapter = render_app.world().resource::<RenderAdapter>();
        let adapter_downlevel_flags = render_adapter.get_downlevel_capabilities().flags;

        render_app.insert_resource(GpuAllocator {
            slabs: HashMap::default(),
            slab_size: self.slab_size,
            next_slab_id: SlabId::default(),
            classes: HashMap::default(),
            adapter_downlevel_flags,
        });
    }
}

impl Default for GpuAllocatorPlugin {
    fn default() -> Self {
        Self {
            slab_size: DEFAULT_SLAB_SIZE,
        }
    }
}

impl Drop for GpuAllocation {
    fn drop(&mut self) {
        // This should never happen, but if it does, we're in a destructor, so
        // let's not abort the process.
        let Ok(mut class_allocator) = self.class_allocator.write() else {
            error!("Couldn't lock the class allocator; just leaking");
            return;
        };

        // Free the allocation.
        match self.allocation_id {
            GpuAllocationId::Regular(regular_slab_id, allocation) => {
                // Find the slab that this allocation came from.
                let Some((ref mut allocator, _)) =
                    class_allocator.regular_slabs.get_mut(regular_slab_id)
                else {
                    error!(
                        "Couldn't find the slab that this allocation came from; just leaking. \
                        (Is the allocator corrupt?)"
                    );
                    return;
                };

                // Tell the allocator to mark this allocation as free.
                allocator.free(allocation);
            }

            GpuAllocationId::Large(large_slab_id) => {
                // Find the slab that this allocation came from.
                let Some(slab) = class_allocator.large_slabs.remove(large_slab_id) else {
                    error!(
                        "Couldn't find the slab that this allocation came from; just leaking. \
                        (Is the allocator corrupt?)"
                    );
                    return;
                };

                // Mark the slab as free.
                class_allocator.free_large_slabs.push(slab);
            }
        }
    }
}

impl GpuAllocationClass {
    /// Returns the number of bytes of storage that a single element of this
    /// class uses.
    fn unit_size(&self) -> u32 {
        match *self {
            GpuAllocationClass::VertexBuffer(ref layout) => layout.0.layout().array_stride as u32,
            GpuAllocationClass::IndexBuffer(IndexFormat::Uint16) => 2,
            GpuAllocationClass::IndexBuffer(IndexFormat::Uint32) => 4,
        }
    }

    /// Returns the number of bytes of storage that a single element of this
    /// class uses, rounded up to the nearest 4 bytes.
    ///
    /// This exists because copies in `wgpu` must begin and end on 4-byte
    /// boundaries, so we have to pad out allocations appropriately.
    fn aligned_unit_size(&self) -> u32 {
        let mut unit_size = self.unit_size();
        if unit_size % 4 != 0 {
            unit_size += 4 - unit_size % 4;
        }
        unit_size
    }
}

impl GpuAllocator {
    /// Returns the slab that the given allocation is stored in.
    pub fn buffer(&self, allocation: &GpuAllocation) -> &Buffer {
        &self.slabs[&allocation.slab_id]
    }
}

impl GpuAllocation {
    /// Returns the location within the slab of the allocation, *in elements,
    /// not in bytes*.
    pub fn offset(&self) -> u32 {
        self.offset
    }

    /// Returns the ID of the slab that this allocation is stored in.
    pub fn slab_id(&self) -> SlabId {
        self.slab_id
    }
}

impl GpuAllocator {
    /// Allocates memory of the given [`GpuAllocationClass`], and copies data
    /// into it.
    ///
    /// New slabs are automatically allocated, so this method can't fail.
    pub fn allocate_with(
        &mut self,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
        class: &GpuAllocationClass,
        contents: &[u8],
    ) -> GpuAllocation {
        // If this is going to overflow a slab, give it
        if self.class_requires_large_allocation(class) || (contents.len() as u64) > self.slab_size {
            return self.allocate_large_with(render_device, class, contents);
        }

        let mut found_allocation = None;

        {
            // Create the class allocator if we need to.
            let class_allocator = self.classes.entry(class.clone()).or_insert_with(default);
            let mut class_allocator_data = class_allocator
                .write()
                .expect("Failed to lock the class allocator for writing");

            // Align up to the nearest 4 bytes so we can copy in.
            let (unit_size, aligned_unit_size) = (class.unit_size(), class.aligned_unit_size());
            let aligned_contents_size = contents.len().div_ceil(aligned_unit_size as usize) as u32;

            // Try to allocate in one of our existing slabs with a simple first-fit
            // algorithm.
            for (regular_slab_id, (ref mut allocator, slab_id)) in
                class_allocator_data.regular_slabs.iter_mut()
            {
                if let Some(allocation) = allocator.allocate(aligned_contents_size) {
                    found_allocation = Some(GpuAllocation {
                        allocation_id: GpuAllocationId::Regular(regular_slab_id, allocation),
                        offset: (allocation.offset as u64 * aligned_unit_size as u64
                            / unit_size as u64) as u32,
                        slab_id: *slab_id,
                        class_allocator: class_allocator.clone(),
                    });
                    break;
                }
            }
        }

        // If we couldn't allocate in any of our existing slabs, create a new
        // one.
        let allocation = found_allocation
            .unwrap_or_else(|| self.allocate_new_regular_slab(render_device, class, contents));

        // Copy data in. Pad out data to be a multiple of 4 bytes in size if
        // necessary. (It's unfortunate that we incur a copy in that case…)
        let buffer = &self.slabs[&allocation.slab_id];
        let byte_offset = allocation.offset() as u64 * class.unit_size() as u64;
        if contents.len() % 4 == 0 {
            render_queue.write_buffer(buffer, byte_offset, contents);
        } else {
            let contents = contents
                .iter()
                .copied()
                .chain(iter::repeat(0).take(4 - (contents.len() % 4)))
                .collect::<Vec<_>>();

            render_queue.write_buffer(buffer, byte_offset, &contents);
        };

        allocation
    }

    /// Allocates memory of the given [`GpuAllocationClass`], giving it its own
    /// slab.
    ///
    /// This is used for allocations that overflow the maximum size of a single
    /// slab, or for allocations that can't be allocated together due to platform limitations.
    fn allocate_large_with(
        &mut self,
        render_device: &RenderDevice,
        class: &GpuAllocationClass,
        contents: &[u8],
    ) -> GpuAllocation {
        // Create a class if we need to.
        let class_allocator = self.classes.entry(class.clone()).or_insert_with(default);
        let mut class_allocator_data = class_allocator
            .write()
            .expect("Failed to lock the class allocator for writing");

        // Try to see if we can reuse an existing large slab.
        let mut slab_id = None;
        for slab_index in 0..class_allocator_data.free_large_slabs.len() {
            if self.slabs[&class_allocator_data.free_large_slabs[slab_index]].size()
                >= contents.len() as u64 * 4
            {
                slab_id = Some(
                    class_allocator_data
                        .free_large_slabs
                        .swap_remove(slab_index),
                );
                break;
            }
        }

        // If we couldn't, create a new slab.
        let slab_id = slab_id.unwrap_or_else(|| {
            let slab_id = self.next_slab_id;
            *self.next_slab_id += 1;
            self.slabs.insert(
                slab_id,
                render_device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some(&format!("large slab ({})", class)),
                    contents,
                    usage: class.buffer_usage(),
                }),
            );
            slab_id
        });

        // Create a large slab ID so we can track this allocation, and return.
        let large_slab_id = class_allocator_data.large_slabs.insert(slab_id);

        GpuAllocation {
            allocation_id: GpuAllocationId::Large(large_slab_id),
            offset: 0,
            slab_id,
            class_allocator: class_allocator.clone(),
        }
    }

    /// Creates a new regular slab containing a single allocation and copies data into it.
    fn allocate_new_regular_slab(
        &mut self,
        render_device: &RenderDevice,
        class: &GpuAllocationClass,
        contents: &[u8],
    ) -> GpuAllocation {
        // Look up the per-class allocator.
        let class_allocator = &self.classes[class];
        let mut class_allocator_data = class_allocator
            .write()
            .expect("Failed to lock the class allocator for writing");

        let (unit_size, aligned_unit_size) = (class.unit_size(), class.aligned_unit_size());
        let aligned_contents_size = contents.len().div_ceil(aligned_unit_size as usize) as u32;

        // Create the buffer. We the buffer to have `COPY_DST` so we can copy
        // data into it.
        let buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some(&format!("regular slab ({})", class)),
            size: self.slab_size,
            usage: class.buffer_usage() | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create a new slab ID.
        let slab_id = self.next_slab_id;
        *self.next_slab_id += 1;
        self.slabs.insert(slab_id, buffer);

        // Create the allocator.
        let mut allocator = Allocator::new((self.slab_size / aligned_unit_size as u64) as u32);

        // Perform the initial allocation.
        let allocation = allocator
            .allocate(aligned_contents_size)
            .expect("Initial allocation should never fail");
        let regular_slab_id = class_allocator_data
            .regular_slabs
            .insert((allocator, slab_id));

        GpuAllocation {
            allocation_id: GpuAllocationId::Regular(regular_slab_id, allocation),
            offset: (allocation.offset as u64 * aligned_unit_size as u64 / unit_size as u64) as u32,
            slab_id,
            class_allocator: class_allocator.clone(),
        }
    }

    /// Returns true if the given allocation class requires its own slab due to platform limitations.
    fn class_requires_large_allocation(&self, class: &GpuAllocationClass) -> bool {
        match *class {
            GpuAllocationClass::IndexBuffer(_) => false,
            GpuAllocationClass::VertexBuffer(_) => !self
                .adapter_downlevel_flags
                .contains(DownlevelFlags::BASE_VERTEX),
        }
    }
}

impl GpuClassAllocatorData {
    /// Returns true if all slabs are empty.
    fn is_empty(&self) -> bool {
        self.regular_slabs.is_empty() && self.large_slabs.is_empty()
    }
}

/// A system that runs every [`SWEEP_INTERVAL`] seconds and returns unused slab
/// memory to the GPU.
fn free_unused_slabs(allocator: ResMut<GpuAllocator>) {
    let allocator = allocator.into_inner();
    let slab_size = allocator.slab_size;

    // Gather up a list of slabs to delete. We'll delete them all at once after
    // this.
    let mut slabs_to_free = vec![];
    allocator.classes.retain(|_, class| {
        let Ok(mut class) = class.write() else {
            return true;
        };

        // Free regular slabs.
        class.regular_slabs.retain(|_, (allocator, slab_id)| {
            // The slab is free if it contains maximal free space.
            let free = allocator.storage_report().total_free_space as u64 == slab_size;
            if free {
                slabs_to_free.push(*slab_id);
            }
            !free
        });

        // Free large slabs.
        slabs_to_free.append(&mut class.free_large_slabs);

        // If the class is now entirely empty, delete it.
        !class.is_empty()
    });

    for slab in slabs_to_free {
        if let Some(buffer) = allocator.slabs.remove(&slab) {
            buffer.destroy();
        }
    }
}

impl Display for GpuAllocationClass {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            GpuAllocationClass::VertexBuffer(ref layout) => {
                let mut hasher = DefaultHasher::new();
                layout.0.hash(&mut hasher);
                let hash = hasher.finish();
                write!(f, "vertex buffer ({:16x})", hash)
            }
            GpuAllocationClass::IndexBuffer(IndexFormat::Uint16) => {
                f.write_str("index buffer (u16)")
            }
            GpuAllocationClass::IndexBuffer(IndexFormat::Uint32) => {
                f.write_str("index buffer (u32)")
            }
        }
    }
}

impl GpuAllocationClass {
    /// Returns the `wgpu` [`BufferUsages`] that slabs storing allocations of
    /// this type must have.
    fn buffer_usage(&self) -> BufferUsages {
        match *self {
            GpuAllocationClass::VertexBuffer(_) => BufferUsages::VERTEX,
            GpuAllocationClass::IndexBuffer(_) => BufferUsages::INDEX,
        }
    }
}

impl Debug for GpuAllocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} @ {:?}", self.allocation_id, self.slab_id)
    }
}

impl Debug for GpuAllocationId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Regular(slab, allocation) => write!(f, "R({:?}, {})", slab, allocation.offset),
            Self::Large(slab) => write!(f, "L({:?})", slab),
        }
    }
}
