//! GPU memory buffer allocation.

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
use wgpu::{util::BufferInitDescriptor, BufferDescriptor, BufferUsages, IndexFormat};

use crate::{
    mesh::MeshVertexBufferLayoutRef,
    render_resource::Buffer,
    renderer::{RenderDevice, RenderQueue},
    Render, RenderApp,
};

const SWEEP_INTERVAL: Duration = Duration::from_secs(10);

pub struct GpuAllocatorPlugin {
    slab_size: u64,
}

#[derive(Resource, Clone)]
pub struct GpuAllocator {
    slabs: HashMap<SlabId, Buffer>,
    slab_size: u64,
    next_slab_id: SlabId,
    classes: HashMap<GpuAllocationClass, GpuClassAllocator>,
}

#[derive(Clone, Default, Deref, DerefMut)]
struct GpuClassAllocator(Arc<RwLock<GpuClassAllocatorData>>);

#[derive(Default)]
struct GpuClassAllocatorData {
    regular_slabs: SlotMap<RegularSlabId, (Allocator, SlabId)>,
    large_slabs: SlotMap<LargeSlabId, SlabId>,
    free_large_slabs: Vec<SlabId>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum GpuAllocationClass {
    VertexBuffer(MeshVertexBufferLayoutRef),
    IndexBuffer(IndexFormat),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Deref, DerefMut, Debug)]
#[repr(transparent)]
pub struct SlabId(pub u32);

new_key_type! {
    pub struct RegularSlabId;
}

new_key_type! {
    pub struct LargeSlabId;
}

#[derive(Clone)]
pub struct GpuAllocation {
    allocation_id: GpuAllocationId,
    slab_id: SlabId,

    /// This is the offset in `unit_size` elements. It may differ from the
    /// offset in `Allocation` because the one in `Allocation` is in multiples
    /// of `aligned_unit_size`, while this one is in multiples of `unit_size`.
    offset: u32,

    // Only needed when dropping.
    class_allocator: GpuClassAllocator,
}

#[derive(Clone)]
enum GpuAllocationId {
    Regular(RegularSlabId, Allocation),
    Large(LargeSlabId),
}

impl Plugin for GpuAllocatorPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .insert_resource(GpuAllocator {
                slabs: HashMap::default(),
                slab_size: self.slab_size,
                next_slab_id: SlabId::default(),
                classes: HashMap::default(),
            })
            .add_systems(Render, free_unused_slabs.run_if(on_timer(SWEEP_INTERVAL)));
    }
}

impl Default for GpuAllocatorPlugin {
    fn default() -> Self {
        // 32 MB slabs by default.
        Self {
            slab_size: 32 * 1024 * 1024,
        }
    }
}

impl Drop for GpuAllocation {
    fn drop(&mut self) {
        // This is written to avoid panics.
        let Ok(mut class_allocator) = self.class_allocator.write() else {
            error!("Couldn't lock the class allocator; just leaking");
            return;
        };

        match self.allocation_id {
            GpuAllocationId::Regular(regular_slab_id, allocation) => {
                let Some((ref mut allocator, _)) =
                    class_allocator.regular_slabs.get_mut(regular_slab_id)
                else {
                    error!(
                        "Couldn't find the slab that this allocation came from; just leaking. \
                        (Is the allocator corrupt?)"
                    );
                    return;
                };

                // Free the allocation.
                allocator.free(allocation);
            }

            GpuAllocationId::Large(large_slab_id) => {
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
    fn unit_size(&self) -> u32 {
        match *self {
            GpuAllocationClass::VertexBuffer(ref layout) => layout.0.layout().array_stride as u32,
            GpuAllocationClass::IndexBuffer(IndexFormat::Uint16) => 2,
            GpuAllocationClass::IndexBuffer(IndexFormat::Uint32) => 4,
        }
    }

    fn aligned_unit_size(&self) -> u32 {
        let mut unit_size = self.unit_size();
        if unit_size % 4 != 0 {
            unit_size += 4 - unit_size % 4;
        }
        unit_size
    }
}

impl GpuAllocator {
    pub fn buffer(&self, allocation: &GpuAllocation) -> &Buffer {
        &self.slabs[&allocation.slab_id]
    }
}

impl GpuAllocation {
    /// This is in elements, not bytes.
    pub fn offset(&self) -> u32 {
        self.offset
    }

    pub fn slab_id(&self) -> SlabId {
        self.slab_id
    }
}

impl GpuAllocator {
    pub fn allocate_with(
        &mut self,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
        class: &GpuAllocationClass,
        contents: &[u8],
    ) -> GpuAllocation {
        if (contents.len() as u64) > self.slab_size {
            return self.allocate_large_with(render_device, class, contents);
        }

        let class_allocator = self.classes.entry(class.clone()).or_insert_with(default);

        let mut class_allocator_data = class_allocator
            .write()
            .expect("Failed to lock the class allocator for writing");

        let (unit_size, aligned_unit_size) = (class.unit_size(), class.aligned_unit_size());
        let aligned_contents_size = contents.len().div_ceil(aligned_unit_size as usize) as u32;

        // First-fit.
        let mut found_allocation = None;
        for (regular_slab_id, (ref mut allocator, slab_id)) in
            class_allocator_data.regular_slabs.iter_mut()
        {
            if let Some(allocation) = allocator.allocate(aligned_contents_size) {
                found_allocation = Some(GpuAllocation {
                    allocation_id: GpuAllocationId::Regular(regular_slab_id, allocation),
                    offset: (allocation.offset as u64 * aligned_unit_size as u64 / unit_size as u64)
                        as u32,
                    slab_id: *slab_id,
                    class_allocator: class_allocator.clone(),
                });
                break;
            }
        }

        let allocation = found_allocation.unwrap_or_else(|| {
            // We need buffers to have `COPY_DST` so we can, well, copy data
            // into them.
            let buffer = render_device.create_buffer(&BufferDescriptor {
                label: Some(&format!("regular slab ({})", class)),
                size: self.slab_size,
                usage: class.buffer_usage() | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let slab_id = self.next_slab_id;
            *self.next_slab_id += 1;
            self.slabs.insert(slab_id, buffer);

            let mut allocator = Allocator::new((self.slab_size / aligned_unit_size as u64) as u32);
            let allocation = allocator
                .allocate(aligned_contents_size)
                .expect("Initial allocation should never fail");
            let regular_slab_id = class_allocator_data
                .regular_slabs
                .insert((allocator, slab_id));

            GpuAllocation {
                allocation_id: GpuAllocationId::Regular(regular_slab_id, allocation),
                offset: (allocation.offset as u64 * aligned_unit_size as u64 / unit_size as u64)
                    as u32,
                slab_id,
                class_allocator: class_allocator.clone(),
            }
        });

        // Copy data in. Pad out data to be a multiple of 4 bytes in size if
        // necessary. (This is unfortunate!)
        let buffer = &self.slabs[&allocation.slab_id];
        let byte_offset = allocation.offset() as u64 * unit_size as u64;
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

    pub fn allocate_large_with(
        &mut self,
        render_device: &RenderDevice,
        class: &GpuAllocationClass,
        contents: &[u8],
    ) -> GpuAllocation {
        let class_allocator = self.classes.entry(class.clone()).or_insert_with(default);

        let mut class_allocator_data = class_allocator
            .write()
            .expect("Failed to lock the class allocator for writing");

        // First-fit.
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

        let large_slab_id = class_allocator_data.large_slabs.insert(slab_id);

        GpuAllocation {
            allocation_id: GpuAllocationId::Large(large_slab_id),
            offset: 0,
            slab_id,
            class_allocator: class_allocator.clone(),
        }
    }
}

impl GpuClassAllocatorData {
    fn is_empty(&self) -> bool {
        self.regular_slabs.is_empty() && self.large_slabs.is_empty()
    }
}

fn free_unused_slabs(allocator: ResMut<GpuAllocator>) {
    let allocator = allocator.into_inner();
    let slab_size = allocator.slab_size;
    let mut slabs_to_free = vec![];

    allocator.classes.retain(|_, class| {
        let Ok(mut class) = class.write() else {
            return true;
        };

        // Free regular slabs.
        class.regular_slabs.retain(|_, (allocator, slab_id)| {
            let free = allocator.storage_report().total_free_space as u64 == slab_size;
            if free {
                slabs_to_free.push(*slab_id);
            }
            !free
        });

        // Free large slabs.
        slabs_to_free.append(&mut class.free_large_slabs);

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
