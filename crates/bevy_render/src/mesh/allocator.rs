//! Manages mesh vertex and index buffers.

use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
    iter,
    ops::Range,
    vec::Vec,
};

use bevy_app::{App, Plugin};
use bevy_asset::AssetId;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    schedule::IntoSystemConfigs as _,
    system::{Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_utils::{
    hashbrown::{HashMap, HashSet},
    tracing::error,
};
use offset_allocator::{Allocation, Allocator};
use wgpu::{
    util::BufferInitDescriptor, BufferDescriptor, BufferUsages, CommandEncoderDescriptor,
    DownlevelFlags, COPY_BUFFER_ALIGNMENT,
};

use crate::{
    mesh::{Indices, Mesh, MeshVertexBufferLayouts, RenderMesh},
    render_asset::{prepare_assets, ExtractedAssets},
    render_resource::Buffer,
    renderer::{RenderAdapter, RenderDevice, RenderQueue},
    Render, RenderApp, RenderSet,
};

/// A plugin that manages GPU memory for mesh data.
pub struct MeshAllocatorPlugin;

/// Manages the assignment of mesh data to GPU buffers.
///
/// The Bevy renderer tries to pack vertex and index data for multiple meshes
/// together so that multiple meshes can be drawn back-to-back without any
/// rebinding. This resource manages these buffers.
///
/// Within each slab, or hardware buffer, the underlying allocation algorithm is
/// [`offset-allocator`], a Rust port of Sebastian Aaltonen's hard-real-time C++
/// `OffsetAllocator`. Slabs start small and then grow as their contents fill
/// up, up to a maximum size limit. To reduce fragmentation, vertex and index
/// buffers that are too large bypass this system and receive their own buffers.
///
/// The [`MeshAllocatorSettings`] allows you to tune the behavior of the
/// allocator for better performance with your application. Most applications
/// won't need to change the settings from their default values.
#[derive(Resource)]
pub struct MeshAllocator {
    /// Holds all buffers and allocators.
    slabs: HashMap<SlabId, Slab>,

    /// Maps a layout to the slabs that hold elements of that layout.
    ///
    /// This is used when allocating, so that we can find the appropriate slab
    /// to place an object in.
    slab_layouts: HashMap<ElementLayout, Vec<SlabId>>,

    /// Maps mesh asset IDs to the ID of the slabs that hold their vertex data.
    mesh_id_to_vertex_slab: HashMap<AssetId<Mesh>, SlabId>,

    /// Maps mesh asset IDs to the ID of the slabs that hold their index data.
    mesh_id_to_index_slab: HashMap<AssetId<Mesh>, SlabId>,

    /// The next slab ID to assign.
    next_slab_id: SlabId,

    /// Whether we can pack multiple vertex arrays into a single slab on this
    /// platform.
    ///
    /// This corresponds to [`DownlevelFlags::BASE_VERTEX`], which is unset on
    /// WebGL 2. On this platform, we must give each vertex array its own
    /// buffer, because we can't adjust the first vertex when we perform a draw.
    general_vertex_slabs_supported: bool,
}

/// Tunable parameters that customize the behavior of the allocator.
///
/// Generally, these parameters adjust the tradeoff between memory fragmentation
/// and performance. You can adjust them as desired for your application. Most
/// applications can stick with the default values.
#[derive(Resource)]
pub struct MeshAllocatorSettings {
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
    /// If a mesh has vertex or index data that exceeds this size limit, that
    /// data is placed in its own slab. This reduces fragmentation, but incurs
    /// more CPU-side binding overhead when drawing the mesh.
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

impl Default for MeshAllocatorSettings {
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

/// The hardware buffer that mesh data lives in, as well as the range within
/// that buffer.
pub struct MeshBufferSlice<'a> {
    /// The buffer that the mesh data resides in.
    pub buffer: &'a Buffer,

    /// The range of elements within this buffer that the mesh data resides in,
    /// measured in elements.
    ///
    /// This is not a byte range; it's an element range. For vertex data, this
    /// is measured in increments of a single vertex. (Thus, if a vertex is 32
    /// bytes long, then this range is in units of 32 bytes each.) For index
    /// data, this is measured in increments of a single index value (2 or 4
    /// bytes). Draw commands generally take their ranges in elements, not
    /// bytes, so this is the most convenient unit in this case.
    pub range: Range<u32>,
}

/// The index of a single slab.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
struct SlabId(u32);

/// Data for a single slab.
#[allow(clippy::large_enum_variant)]
enum Slab {
    /// A slab that can contain multiple objects.
    General(GeneralSlab),
    /// A slab that contains a single object.
    LargeObject(LargeObjectSlab),
}

/// A resizable slab that can contain multiple objects.
///
/// This is the normal type of slab used for objects that are below the
/// [`MeshAllocatorSettings::large_threshold`]. Slabs are divided into *slots*,
/// which are described in detail in the [`ElementLayout`] documentation.
struct GeneralSlab {
    /// The [`Allocator`] that manages the objects in this slab.
    allocator: Allocator,

    /// The GPU buffer that backs this slab.
    ///
    /// This may be `None` if the buffer hasn't been created yet. We delay
    /// creation of buffers until allocating all the meshes for a single frame,
    /// so that we don't needlessly create and resize buffers when many meshes
    /// load all at once.
    buffer: Option<Buffer>,

    /// Allocations that are on the GPU.
    ///
    /// The range is in slots.
    resident_allocations: HashMap<AssetId<Mesh>, SlabAllocation>,

    /// Allocations that are waiting to be uploaded to the GPU.
    ///
    /// The range is in slots.
    pending_allocations: HashMap<AssetId<Mesh>, SlabAllocation>,

    /// The layout of a single element (vertex or index).
    element_layout: ElementLayout,

    /// The size of this slab in slots.
    slot_capacity: u32,
}

/// A slab that contains a single object.
///
/// Typically, this is for objects that exceed the
/// [`MeshAllocatorSettings::large_threshold`]. This is also for objects that
/// would ordinarily receive their own slab but can't because of platform
/// limitations, most notably vertex arrays on WebGL 2.
struct LargeObjectSlab {
    /// The GPU buffer that backs this slab.
    ///
    /// This may be `None` if the buffer hasn't been created yet.
    buffer: Option<Buffer>,

    /// The layout of a single element (vertex or index).
    element_layout: ElementLayout,
}

/// The type of element that a slab can store.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum ElementClass {
    /// Data for a vertex.
    Vertex,
    /// A vertex index.
    Index,
}

/// Information about the size of individual elements (vertices or indices)
/// within a slab.
///
/// Slab objects are allocated in units of *slots*. Usually, each element takes
/// up one slot, and so elements and slots are equivalent. Occasionally,
/// however, a slot may consist of 2 or even 4 elements. This occurs when the
/// size of an element isn't divisible by [`COPY_BUFFER_ALIGNMENT`]. When we
/// resize buffers, we perform GPU-to-GPU copies to shuffle the existing
/// elements into their new positions, and such copies must be on
/// [`COPY_BUFFER_ALIGNMENT`] boundaries. Slots solve this problem by
/// guaranteeing that the size of an allocation quantum is divisible by both the
/// size of an element and [`COPY_BUFFER_ALIGNMENT`], so we can relocate it
/// freely.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ElementLayout {
    /// Either a vertex or an index.
    class: ElementClass,

    /// The size in bytes of a single element (vertex or index).
    size: u64,

    /// The number of elements that make up a single slot.
    ///
    /// Usually, this is 1, but it can be different if [`ElementLayout::size`]
    /// isn't divisible by 4. See the comment in [`ElementLayout`] for more
    /// details.
    elements_per_slot: u32,
}

/// The location of an allocation and the slab it's contained in.
struct MeshAllocation {
    /// The ID of the slab.
    slab_id: SlabId,
    /// Holds the actual allocation.
    slab_allocation: SlabAllocation,
}

/// An allocation within a slab.
#[derive(Clone)]
struct SlabAllocation {
    /// The actual [`Allocator`] handle, needed to free the allocation.
    allocation: Allocation,
    /// The number of slots that this allocation takes up.
    slot_count: u32,
}

/// Holds information about all slabs scheduled to be allocated or reallocated.
#[derive(Default, Deref, DerefMut)]
struct SlabsToReallocate(HashMap<SlabId, SlabToReallocate>);

/// Holds information about a slab that's scheduled to be allocated or
/// reallocated.
#[derive(Default)]
struct SlabToReallocate {
    /// Maps all allocations that need to be relocated to their positions within
    /// the *new* slab.
    allocations_to_copy: HashMap<AssetId<Mesh>, SlabAllocation>,
}

impl Display for SlabId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Plugin for MeshAllocatorPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<MeshAllocatorSettings>()
            .add_systems(
                Render,
                allocate_and_free_meshes
                    .in_set(RenderSet::PrepareAssets)
                    .before(prepare_assets::<RenderMesh>),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        // The `RenderAdapter` isn't available until now, so we can't do this in
        // [`Plugin::build`].
        render_app.init_resource::<MeshAllocator>();
    }
}

impl FromWorld for MeshAllocator {
    fn from_world(world: &mut World) -> Self {
        // Note whether we're on WebGL 2. In this case, we must give every
        // vertex array its own slab.
        let render_adapter = world.resource::<RenderAdapter>();
        let general_vertex_slabs_supported = render_adapter
            .get_downlevel_capabilities()
            .flags
            .contains(DownlevelFlags::BASE_VERTEX);

        Self {
            slabs: HashMap::new(),
            slab_layouts: HashMap::new(),
            mesh_id_to_vertex_slab: HashMap::new(),
            mesh_id_to_index_slab: HashMap::new(),
            next_slab_id: SlabId(0),
            general_vertex_slabs_supported,
        }
    }
}

/// A system that processes newly-extracted or newly-removed meshes and writes
/// their data into buffers or frees their data as appropriate.
pub fn allocate_and_free_meshes(
    mut mesh_allocator: ResMut<MeshAllocator>,
    mesh_allocator_settings: Res<MeshAllocatorSettings>,
    extracted_meshes: Res<ExtractedAssets<RenderMesh>>,
    mut mesh_vertex_buffer_layouts: ResMut<MeshVertexBufferLayouts>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    // Process newly-added meshes.
    mesh_allocator.allocate_meshes(
        &mesh_allocator_settings,
        &extracted_meshes,
        &mut mesh_vertex_buffer_layouts,
        &render_device,
        &render_queue,
    );

    // Process removed meshes.
    mesh_allocator.free_meshes(&extracted_meshes);
}

impl MeshAllocator {
    /// Returns the buffer and range within that buffer of the vertex data for
    /// the mesh with the given ID.
    ///
    /// If the mesh wasn't allocated, returns None.
    pub fn mesh_vertex_slice(&self, mesh_id: &AssetId<Mesh>) -> Option<MeshBufferSlice> {
        self.mesh_slice_in_slab(mesh_id, *self.mesh_id_to_vertex_slab.get(mesh_id)?)
    }

    /// Returns the buffer and range within that buffer of the index data for
    /// the mesh with the given ID.
    ///
    /// If the mesh has no index data or wasn't allocated, returns None.
    pub fn mesh_index_slice(&self, mesh_id: &AssetId<Mesh>) -> Option<MeshBufferSlice> {
        self.mesh_slice_in_slab(mesh_id, *self.mesh_id_to_index_slab.get(mesh_id)?)
    }

    /// Given a slab and a mesh with data located with it, returns the buffer
    /// and range of that mesh data within the slab.
    fn mesh_slice_in_slab(
        &self,
        mesh_id: &AssetId<Mesh>,
        slab_id: SlabId,
    ) -> Option<MeshBufferSlice> {
        match self.slabs.get(&slab_id)? {
            Slab::General(ref general_slab) => {
                let slab_allocation = general_slab.resident_allocations.get(mesh_id)?;
                Some(MeshBufferSlice {
                    buffer: general_slab.buffer.as_ref()?,
                    range: (slab_allocation.allocation.offset
                        * general_slab.element_layout.elements_per_slot)
                        ..((slab_allocation.allocation.offset + slab_allocation.slot_count)
                            * general_slab.element_layout.elements_per_slot),
                })
            }

            Slab::LargeObject(ref large_object_slab) => {
                let buffer = large_object_slab.buffer.as_ref()?;
                Some(MeshBufferSlice {
                    buffer,
                    range: 0..((buffer.size() / large_object_slab.element_layout.size) as u32),
                })
            }
        }
    }

    /// Processes newly-loaded meshes, allocating room in the slabs for their
    /// mesh data and performing upload operations as appropriate.
    fn allocate_meshes(
        &mut self,
        mesh_allocator_settings: &MeshAllocatorSettings,
        extracted_meshes: &ExtractedAssets<RenderMesh>,
        mesh_vertex_buffer_layouts: &mut MeshVertexBufferLayouts,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) {
        let mut slabs_to_grow = SlabsToReallocate::default();

        // Allocate.
        for (mesh_id, mesh) in &extracted_meshes.extracted {
            // Allocate vertex data. Note that we can only pack mesh vertex data
            // together if the platform supports it.
            let vertex_element_layout = ElementLayout::vertex(mesh_vertex_buffer_layouts, mesh);
            if self.general_vertex_slabs_supported {
                self.allocate(
                    mesh_id,
                    mesh.get_vertex_buffer_data().len() as u64,
                    vertex_element_layout,
                    &mut slabs_to_grow,
                    mesh_allocator_settings,
                );
            } else {
                self.allocate_large(mesh_id, vertex_element_layout);
            }

            // Allocate index data.
            if let (Some(index_buffer_data), Some(index_element_layout)) =
                (mesh.get_index_buffer_bytes(), ElementLayout::index(mesh))
            {
                self.allocate(
                    mesh_id,
                    index_buffer_data.len() as u64,
                    index_element_layout,
                    &mut slabs_to_grow,
                    mesh_allocator_settings,
                );
            }
        }

        // Perform growth.
        for (slab_id, slab_to_grow) in slabs_to_grow.0 {
            self.reallocate_slab(render_device, render_queue, slab_id, slab_to_grow);
        }

        // Copy new mesh data in.
        for (mesh_id, mesh) in &extracted_meshes.extracted {
            self.copy_mesh_vertex_data(mesh_id, mesh, render_device, render_queue);
            self.copy_mesh_index_data(mesh_id, mesh, render_device, render_queue);
        }
    }

    /// Copies vertex array data from a mesh into the appropriate spot in the
    /// slab.
    fn copy_mesh_vertex_data(
        &mut self,
        mesh_id: &AssetId<Mesh>,
        mesh: &Mesh,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) {
        let Some(&slab_id) = self.mesh_id_to_vertex_slab.get(mesh_id) else {
            return;
        };
        let vertex_data = mesh.get_vertex_buffer_data();

        // Call the generic function.
        self.copy_element_data(
            mesh_id,
            mesh,
            &vertex_data,
            BufferUsages::VERTEX,
            slab_id,
            render_device,
            render_queue,
        );
    }

    /// Copies index array data from a mesh into the appropriate spot in the
    /// slab.
    fn copy_mesh_index_data(
        &mut self,
        mesh_id: &AssetId<Mesh>,
        mesh: &Mesh,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) {
        let Some(&slab_id) = self.mesh_id_to_index_slab.get(mesh_id) else {
            return;
        };
        let Some(index_data) = mesh.get_index_buffer_bytes() else {
            return;
        };

        // Call the generic function.
        self.copy_element_data(
            mesh_id,
            mesh,
            index_data,
            BufferUsages::INDEX,
            slab_id,
            render_device,
            render_queue,
        );
    }

    /// A generic function that copies either vertex or index data into a slab.
    #[allow(clippy::too_many_arguments)]
    fn copy_element_data(
        &mut self,
        mesh_id: &AssetId<Mesh>,
        mesh: &Mesh,
        data: &[u8],
        buffer_usages: BufferUsages,
        slab_id: SlabId,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) {
        let Some(slab) = self.slabs.get_mut(&slab_id) else {
            return;
        };

        match *slab {
            Slab::General(ref mut general_slab) => {
                let (Some(ref buffer), Some(allocated_range)) = (
                    &general_slab.buffer,
                    general_slab.pending_allocations.remove(mesh_id),
                ) else {
                    return;
                };

                let slot_size = general_slab.element_layout.slot_size();

                // Write the data in.
                render_queue.write_buffer(
                    buffer,
                    allocated_range.allocation.offset as u64 * slot_size,
                    &pad_to_alignment(data, slot_size as usize),
                );

                // Mark the allocation as resident.
                general_slab
                    .resident_allocations
                    .insert(*mesh_id, allocated_range);
            }

            Slab::LargeObject(ref mut large_object_slab) => {
                debug_assert!(large_object_slab.buffer.is_none());

                // Create the buffer and its data in one go.
                large_object_slab.buffer = Some(render_device.create_buffer_with_data(
                    &BufferInitDescriptor {
                        label: Some(&format!(
                            "large mesh slab {} ({}buffer)",
                            slab_id,
                            buffer_usages_to_str(buffer_usages)
                        )),
                        contents: &mesh.get_vertex_buffer_data(),
                        usage: buffer_usages | BufferUsages::COPY_DST,
                    },
                ));
            }
        }
    }

    fn free_meshes(&mut self, extracted_meshes: &ExtractedAssets<RenderMesh>) {
        let mut empty_slabs = HashSet::new();
        for mesh_id in &extracted_meshes.removed {
            if let Some(slab_id) = self.mesh_id_to_vertex_slab.remove(mesh_id) {
                self.free_allocation_in_slab(mesh_id, slab_id, &mut empty_slabs);
            }
            if let Some(slab_id) = self.mesh_id_to_index_slab.remove(mesh_id) {
                self.free_allocation_in_slab(mesh_id, slab_id, &mut empty_slabs);
            }
        }

        for empty_slab in empty_slabs {
            self.slabs.remove(&empty_slab);
        }
    }

    /// Given a slab and the ID of a mesh containing data in it, marks the
    /// allocation as free.
    ///
    /// If this results in the slab becoming empty, this function adds the slab
    /// to the `empty_slabs` set.
    fn free_allocation_in_slab(
        &mut self,
        mesh_id: &AssetId<Mesh>,
        slab_id: SlabId,
        empty_slabs: &mut HashSet<SlabId>,
    ) {
        let Some(slab) = self.slabs.get_mut(&slab_id) else {
            return;
        };

        match *slab {
            Slab::General(ref mut general_slab) => {
                let Some(slab_allocation) = general_slab
                    .resident_allocations
                    .remove(mesh_id)
                    .or_else(|| general_slab.pending_allocations.remove(mesh_id))
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

    /// Allocates space for mesh data with the given byte size and layout in the
    /// appropriate slab, creating that slab if necessary.
    fn allocate(
        &mut self,
        mesh_id: &AssetId<Mesh>,
        data_byte_len: u64,
        layout: ElementLayout,
        slabs_to_grow: &mut SlabsToReallocate,
        settings: &MeshAllocatorSettings,
    ) {
        let data_element_count = data_byte_len.div_ceil(layout.size) as u32;
        let data_slot_count = data_element_count.div_ceil(layout.elements_per_slot);

        // If the mesh data is too large for a slab, give it a slab of its own.
        if data_slot_count as u64 * layout.slot_size()
            >= settings.large_threshold.min(settings.max_slab_size)
        {
            self.allocate_large(mesh_id, layout);
        } else {
            self.allocate_general(mesh_id, data_slot_count, layout, slabs_to_grow, settings);
        }
    }

    /// Allocates space for mesh data with the given slot size and layout in the
    /// appropriate general slab.
    fn allocate_general(
        &mut self,
        mesh_id: &AssetId<Mesh>,
        data_slot_count: u32,
        layout: ElementLayout,
        slabs_to_grow: &mut SlabsToReallocate,
        settings: &MeshAllocatorSettings,
    ) {
        let candidate_slabs = self.slab_layouts.entry(layout).or_default();

        // Loop through the slabs that accept elements of the appropriate type
        // and try to allocate the mesh inside them. We go with the first one
        // that succeeds.
        let mut mesh_allocation = None;
        'slab: for &slab_id in &*candidate_slabs {
            loop {
                let Some(Slab::General(ref mut slab)) = self.slabs.get_mut(&slab_id) else {
                    continue 'slab;
                };

                if let Some(allocation) = slab.allocator.allocate(data_slot_count) {
                    mesh_allocation = Some(MeshAllocation {
                        slab_id,
                        slab_allocation: SlabAllocation {
                            allocation,
                            slot_count: data_slot_count,
                        },
                    });
                    break 'slab;
                }

                // Try to grow the slab. If this fails, the slab is full; go on
                // to the next slab.
                match slab.try_grow(settings) {
                    Ok(new_mesh_allocation_records) => {
                        slabs_to_grow.insert(slab_id, new_mesh_allocation_records);
                    }
                    Err(()) => continue 'slab,
                }
            }
        }

        // If we still have no allocation, make a new slab.
        if mesh_allocation.is_none() {
            let new_slab_id = self.next_slab_id;
            self.next_slab_id.0 += 1;

            let new_slab = GeneralSlab::new(
                new_slab_id,
                &mut mesh_allocation,
                settings,
                layout,
                data_slot_count,
            );

            self.slabs.insert(new_slab_id, Slab::General(new_slab));
            candidate_slabs.push(new_slab_id);
            slabs_to_grow.insert(new_slab_id, SlabToReallocate::default());
        }

        let mesh_allocation = mesh_allocation.expect("Should have been able to allocate");

        // Mark the allocation as pending. Don't copy it in just yet; further
        // meshes loaded this frame may result in its final allocation location
        // changing.
        if let Some(Slab::General(ref mut general_slab)) =
            self.slabs.get_mut(&mesh_allocation.slab_id)
        {
            general_slab
                .pending_allocations
                .insert(*mesh_id, mesh_allocation.slab_allocation);
        };

        self.record_allocation(mesh_id, mesh_allocation.slab_id, layout.class);
    }

    /// Allocates an object into its own dedicated slab.
    fn allocate_large(&mut self, mesh_id: &AssetId<Mesh>, layout: ElementLayout) {
        let new_slab_id = self.next_slab_id;
        self.next_slab_id.0 += 1;

        self.record_allocation(mesh_id, new_slab_id, layout.class);

        self.slabs.insert(
            new_slab_id,
            Slab::LargeObject(LargeObjectSlab {
                buffer: None,
                element_layout: layout,
            }),
        );
    }

    /// Reallocates a slab that needs to be resized, or allocates a new slab.
    ///
    /// This performs the actual growth operation that [`GeneralSlab::try_grow`]
    /// scheduled. We do the growth in two phases so that, if a slab grows
    /// multiple times in the same frame, only one new buffer is reallocated,
    /// rather than reallocating the buffer multiple times.
    fn reallocate_slab(
        &mut self,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
        slab_id: SlabId,
        slab_to_grow: SlabToReallocate,
    ) {
        let Some(Slab::General(slab)) = self.slabs.get_mut(&slab_id) else {
            error!("Couldn't find slab {:?} to grow", slab_id);
            return;
        };

        let old_buffer = slab.buffer.take();

        let mut buffer_usages = BufferUsages::COPY_SRC | BufferUsages::COPY_DST;
        match slab.element_layout.class {
            ElementClass::Vertex => buffer_usages |= BufferUsages::VERTEX,
            ElementClass::Index => buffer_usages |= BufferUsages::INDEX,
        };

        // Create the buffer.
        let new_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some(&format!(
                "general mesh slab {} ({}buffer)",
                slab_id,
                buffer_usages_to_str(buffer_usages)
            )),
            size: slab.slot_capacity as u64 * slab.element_layout.slot_size(),
            usage: buffer_usages,
            mapped_at_creation: false,
        });

        slab.buffer = Some(new_buffer.clone());

        // In order to do buffer copies, we need a command encoder.
        let mut encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("slab resize encoder"),
        });

        // If we have no objects to copy over, we're done.
        let Some(old_buffer) = old_buffer else {
            return;
        };

        for (mesh_id, src_slab_allocation) in &mut slab.resident_allocations {
            let Some(dest_slab_allocation) = slab_to_grow.allocations_to_copy.get(mesh_id) else {
                continue;
            };

            encoder.copy_buffer_to_buffer(
                &old_buffer,
                src_slab_allocation.allocation.offset as u64 * slab.element_layout.slot_size(),
                &new_buffer,
                dest_slab_allocation.allocation.offset as u64 * slab.element_layout.slot_size(),
                dest_slab_allocation.slot_count as u64 * slab.element_layout.slot_size(),
            );
            // Now that we've done the copy, we can update the allocation record.
            *src_slab_allocation = dest_slab_allocation.clone();
        }

        let command_buffer = encoder.finish();
        render_queue.submit([command_buffer]);
    }

    /// Records the location of the given newly-allocated mesh data in the
    /// [`Self::mesh_id_to_vertex_slab`] or [`Self::mesh_id_to_index_slab`]
    /// tables as appropriate.
    fn record_allocation(
        &mut self,
        mesh_id: &AssetId<Mesh>,
        slab_id: SlabId,
        element_class: ElementClass,
    ) {
        match element_class {
            ElementClass::Vertex => {
                self.mesh_id_to_vertex_slab.insert(*mesh_id, slab_id);
            }
            ElementClass::Index => {
                self.mesh_id_to_index_slab.insert(*mesh_id, slab_id);
            }
        }
    }
}

impl GeneralSlab {
    /// Creates a new growable slab big enough to hold an single element of
    /// `data_slot_count` size with the given `layout`.
    fn new(
        new_slab_id: SlabId,
        mesh_allocation: &mut Option<MeshAllocation>,
        settings: &MeshAllocatorSettings,
        layout: ElementLayout,
        data_slot_count: u32,
    ) -> GeneralSlab {
        let slab_slot_capacity = (settings.min_slab_size.div_ceil(layout.slot_size()) as u32)
            .max(offset_allocator::ext::min_allocator_size(data_slot_count));

        let mut new_slab = GeneralSlab {
            allocator: Allocator::new(slab_slot_capacity),
            buffer: None,
            resident_allocations: HashMap::new(),
            pending_allocations: HashMap::new(),
            element_layout: layout,
            slot_capacity: slab_slot_capacity,
        };

        // This should never fail.
        if let Some(allocation) = new_slab.allocator.allocate(data_slot_count) {
            *mesh_allocation = Some(MeshAllocation {
                slab_id: new_slab_id,
                slab_allocation: SlabAllocation {
                    slot_count: data_slot_count,
                    allocation,
                },
            });
        }

        new_slab
    }

    /// Attempts to grow a slab that's just run out of space.
    ///
    /// Returns a structure the allocations that need to be relocated if the
    /// growth succeeded. If the slab is full, returns `Err`.
    fn try_grow(&mut self, settings: &MeshAllocatorSettings) -> Result<SlabToReallocate, ()> {
        // In extremely rare cases due to allocator fragmentation, it may happen
        // that we fail to re-insert every object that was in the slab after
        // growing it. Even though this will likely never happen, we use this
        // loop to handle this unlikely event properly if it does.
        'grow: loop {
            let new_slab_slot_capacity = ((self.slot_capacity as f64 * settings.growth_factor)
                .ceil() as u32)
                .min((settings.max_slab_size / self.element_layout.slot_size()) as u32);
            if new_slab_slot_capacity == self.slot_capacity {
                // The slab is full.
                return Err(());
            }

            // Grow the slab.
            self.allocator = Allocator::new(new_slab_slot_capacity);
            self.slot_capacity = new_slab_slot_capacity;

            let mut slab_to_grow = SlabToReallocate::default();

            // Place every resident allocation that was in the old slab in the
            // new slab.
            for (allocated_mesh_id, old_allocation_range) in &self.resident_allocations {
                let allocation_size = old_allocation_range.slot_count;
                match self.allocator.allocate(allocation_size) {
                    Some(allocation) => {
                        slab_to_grow.allocations_to_copy.insert(
                            *allocated_mesh_id,
                            SlabAllocation {
                                allocation,
                                slot_count: allocation_size,
                            },
                        );
                    }
                    None => {
                        // We failed to insert one of the allocations that we
                        // had before.
                        continue 'grow;
                    }
                }
            }

            // Move every allocation that was pending in the old slab to the new
            // slab.
            for slab_allocation in self.pending_allocations.values_mut() {
                let allocation_size = slab_allocation.slot_count;
                match self.allocator.allocate(allocation_size) {
                    Some(allocation) => slab_allocation.allocation = allocation,
                    None => {
                        // We failed to insert one of the allocations that we
                        // had before.
                        continue 'grow;
                    }
                }
            }

            return Ok(slab_to_grow);
        }
    }
}

impl ElementLayout {
    /// Creates an [`ElementLayout`] for mesh data of the given class (vertex or
    /// index) with the given byte size.
    fn new(class: ElementClass, size: u64) -> ElementLayout {
        ElementLayout {
            class,
            size,
            // Make sure that slot boundaries begin and end on
            // `COPY_BUFFER_ALIGNMENT`-byte (4-byte) boundaries.
            elements_per_slot: (COPY_BUFFER_ALIGNMENT / gcd(size, COPY_BUFFER_ALIGNMENT)) as u32,
        }
    }

    fn slot_size(&self) -> u64 {
        self.size * self.elements_per_slot as u64
    }

    /// Creates the appropriate [`ElementLayout`] for the given mesh's vertex
    /// data.
    fn vertex(
        mesh_vertex_buffer_layouts: &mut MeshVertexBufferLayouts,
        mesh: &Mesh,
    ) -> ElementLayout {
        let mesh_vertex_buffer_layout =
            mesh.get_mesh_vertex_buffer_layout(mesh_vertex_buffer_layouts);
        ElementLayout::new(
            ElementClass::Vertex,
            mesh_vertex_buffer_layout.0.layout().array_stride,
        )
    }

    /// Creates the appropriate [`ElementLayout`] for the given mesh's index
    /// data.
    fn index(mesh: &Mesh) -> Option<ElementLayout> {
        let size = match mesh.indices()? {
            Indices::U16(_) => 2,
            Indices::U32(_) => 4,
        };
        Some(ElementLayout::new(ElementClass::Index, size))
    }
}

impl GeneralSlab {
    /// Returns true if this slab is empty.
    fn is_empty(&self) -> bool {
        self.resident_allocations.is_empty() && self.pending_allocations.is_empty()
    }
}

/// Returns the greatest common divisor of the two numbers.
///
/// <https://en.wikipedia.org/wiki/Euclidean_algorithm#Implementations>
fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Ensures that the size of a buffer is a multiple of the given alignment by
/// padding it with zeroes if necessary.
///
/// If the buffer already has the required size, then this function doesn't
/// allocate. Otherwise, it copies the buffer into a new one and writes the
/// appropriate number of zeroes to the end.
fn pad_to_alignment(buffer: &[u8], align: usize) -> Cow<[u8]> {
    if buffer.len() % align == 0 {
        return Cow::Borrowed(buffer);
    }
    let mut buffer = buffer.to_vec();
    buffer.extend(iter::repeat(0).take(align - buffer.len() % align));
    Cow::Owned(buffer)
}

/// Returns a string describing the given buffer usages.
fn buffer_usages_to_str(buffer_usages: BufferUsages) -> &'static str {
    if buffer_usages.contains(BufferUsages::VERTEX) {
        "vertex "
    } else if buffer_usages.contains(BufferUsages::INDEX) {
        "index "
    } else {
        ""
    }
}
