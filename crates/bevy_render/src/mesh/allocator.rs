//! Manages mesh vertex and index buffers.

use alloc::borrow::Cow;
use bevy_app::{App, Plugin};
use bevy_asset::AssetId;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    resource::Resource,
    schedule::IntoScheduleConfigs as _,
    system::{Res, ResMut},
    world::{FromWorld, World},
};
use bevy_log::warn;
use bevy_math::bounding::{Aabb2d, BoundingVolume};
use bevy_mesh::Indices;
#[cfg(feature = "morph")]
use const_shader_layout::ShaderLayout;
use glam::Vec4;
use wgpu::{BufferUsages, DownlevelFlags, COPY_BUFFER_ALIGNMENT};

#[cfg(feature = "morph")]
use bevy_mesh::morph::MorphAttributes;

use crate::{
    mesh::{Mesh, MeshMetadata, MeshVertexBufferLayouts, RenderMesh},
    render_asset::{prepare_assets, ExtractedAssets},
    renderer::{RenderAdapter, RenderDevice, RenderQueue},
    slab_allocator::{
        Slab, SlabAllocationBufferSlice, SlabAllocator, SlabAllocatorSettings, SlabId, SlabItem,
        SlabItemLayout,
    },
    GpuResourceAppExt, Render, RenderApp, RenderSystems,
};

/// A plugin that manages GPU memory for mesh data.
pub struct MeshAllocatorPlugin;

/// Manages the assignment of mesh data to GPU buffers.
///
/// The Bevy renderer tries to pack vertex and index data for multiple meshes
/// together so that multiple meshes can be drawn back-to-back without any
/// rebinding. This resource manages these buffers.
///
/// The [`MeshAllocatorSettings`] allows you to tune the behavior of the
/// allocator for better performance with your use case. Most applications won't
/// need to change the settings from their default values.
///
#[derive(Resource, Deref, DerefMut)]
pub struct MeshAllocator {
    /// Holds all buffers and offset allocators.
    #[deref]
    slab_allocator: SlabAllocator<MeshSlabItem>,

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
/// and speed. You can adjust them as desired for your application. Most
/// applications can stick with the default values.
#[derive(Resource, Deref, DerefMut)]
pub struct MeshAllocatorSettings {
    #[deref]
    pub slab_allocator_settings: SlabAllocatorSettings,

    /// Additional buffer usages to add to any vertex or index buffers created.
    pub extra_buffer_usages: BufferUsages,
}

impl Default for MeshAllocatorSettings {
    fn default() -> MeshAllocatorSettings {
        MeshAllocatorSettings {
            slab_allocator_settings: SlabAllocatorSettings::default(),
            extra_buffer_usages: BufferUsages::empty(),
        }
    }
}

/// The [`ElementLayout`] for morph displacements.
///
/// All morph displacements currently have the same element layout, so we only
/// need one of these.
#[cfg(feature = "morph")]
const MORPH_ATTRIBUTE_ELEMENT_LAYOUT: ElementLayout = ElementLayout {
    class: ElementClass::MorphTarget,
    size: MorphAttributes::SIZE.get(),
    elements_per_slot: 1,
    buffer_usages: ElementClass::MorphTarget.buffer_usages(true),
};

/// The ID of a single slab.
pub type MeshSlabId = SlabId<MeshSlabItem>;

/// The slab buffer and location within that slab in which each mesh is
/// allocated.
pub type MeshBufferSlice<'a> = SlabAllocationBufferSlice<'a, MeshSlabItem>;

/// The [`SlabItem`] implementation that describes the information needed to
/// allocate and free meshes.
pub struct MeshSlabItem;

impl SlabItem for MeshSlabItem {
    type Key = MeshAllocationKey;
    type Layout = ElementLayout;
    fn label() -> Cow<'static, str> {
        "mesh".into()
    }
}

/// IDs of the slabs associated with a single mesh.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct MeshSlabs {
    /// The slab storing the mesh's vertex data.
    pub vertex_slab_id: MeshSlabId,
    /// The slab storing the mesh's index data, if the mesh is indexed.
    pub index_slab_id: Option<MeshSlabId>,
    /// The slab storing the mesh's metadata.
    pub metadata_slab_id: Option<MeshSlabId>,
    /// The slab storing the mesh's morph target displacements, if the mesh has
    /// morph targets.
    #[cfg(feature = "morph")]
    pub morph_target_slab_id: Option<MeshSlabId>,
}

impl Slab<MeshSlabItem> {
    /// Returns the type of buffer that this is: vertex, index, metadata or morph target.
    pub fn element_class(&self) -> ElementClass {
        self.element_layout().class
    }
}

/// The handle used to retrieve a single mesh allocation.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshAllocationKey {
    /// The ID of the mesh asset.
    pub mesh_id: AssetId<Mesh>,
    /// The type of data: vertex data, index data, or morph data.
    pub class: ElementClass,
}

impl MeshAllocationKey {
    /// Creates a new [`MeshAllocationKey`] for the given mesh asset ID and
    /// class.
    pub fn new(mesh_id: AssetId<Mesh>, class: ElementClass) -> Self {
        Self { mesh_id, class }
    }
}

/// The type of element that a mesh slab can store.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementClass {
    /// Per-mesh metadata, except for meshes without `final_aabb` and `final_uv_ranges`.
    Metadata,
    /// Data for a vertex.
    Vertex,
    /// A vertex index.
    Index,
    #[cfg(feature = "morph")]
    /// Displacement data for a morph target.
    MorphTarget,
}

/// Information about the size of individual elements within a slab.
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
pub struct ElementLayout {
    class: ElementClass,

    /// The size in bytes of a single element.
    size: u64,

    /// The number of elements that make up a single slot.
    ///
    /// Usually, this is 1, but it can be different if [`ElementLayout::size`]
    /// isn't divisible by 4. See the comment in [`ElementLayout`] for more
    /// details.
    elements_per_slot: u32,

    buffer_usages: BufferUsages,
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
                    .in_set(RenderSystems::PrepareAssets)
                    .before(prepare_assets::<RenderMesh>),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        // The `RenderAdapter` isn't available until now, so we can't do this in
        // [`Plugin::build`].
        render_app.init_gpu_resource::<MeshAllocator>();
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

        // Take the `extra_buffer_usages` from the mesh allocator settings into
        // account.
        let mesh_allocator_settings = world.resource::<MeshAllocatorSettings>();
        let mut slab_allocator = SlabAllocator::new();
        slab_allocator.extra_buffer_usages |= mesh_allocator_settings.extra_buffer_usages;

        Self {
            slab_allocator,
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
    // Clear the list of meshes displaced by last frame's slab growth.
    mesh_allocator.clear_displaced_keys();

    // Process removed or modified meshes.
    mesh_allocator.free_meshes(&extracted_meshes);

    // Process newly-added or modified meshes.
    mesh_allocator.allocate_meshes(
        &mesh_allocator_settings,
        &extracted_meshes,
        &mut mesh_vertex_buffer_layouts,
        &render_device,
        &render_queue,
    );
}

impl MeshAllocator {
    /// Returns the buffer and range within that buffer of the metadata for
    /// the mesh with the given ID.
    ///
    /// If the mesh wasn't allocated, returns None.
    pub fn mesh_metadata_slice(&self, mesh_id: &AssetId<Mesh>) -> Option<MeshBufferSlice<'_>> {
        self.slab_allocation_slice(
            &MeshAllocationKey::new(*mesh_id, ElementClass::Metadata),
            *self.mesh_id_to_metadata_slab(mesh_id)?,
        )
    }

    /// Meshes that had the buffer under their vertex or index data replaced this frame by a slab
    /// they were already resident in growing, so that anything caching those buffers can rebuild
    /// just the affected entries.
    ///
    /// **It is not the full set of meshes whose buffers changed this frame, and is not meant to be.**
    /// Callers must handle [`ExtractedAssets`] themselves if they want the full set of changed buffers.
    ///
    /// A mesh is yielded twice if both its vertex and its index slab grew, since those are separate
    /// allocations, so deduplicate if repeating the work per mesh would be expensive.
    ///
    /// Morph target and metadata allocations are filtered out.
    ///
    /// See [`SlabAllocator::keys_displaced_by_slab_growth`], which this wraps.
    pub fn meshes_displaced_by_slab_growth(&self) -> impl Iterator<Item = AssetId<Mesh>> {
        self.keys_displaced_by_slab_growth()
            .iter()
            .filter(|key| matches!(key.class, ElementClass::Vertex | ElementClass::Index))
            .map(|key| key.mesh_id)
    }

    /// Returns the buffer and range within that buffer of the vertex data for
    /// the mesh with the given ID.
    ///
    /// If the mesh wasn't allocated, returns None.
    pub fn mesh_vertex_slice(&self, mesh_id: &AssetId<Mesh>) -> Option<MeshBufferSlice<'_>> {
        self.slab_allocation_slice(
            &MeshAllocationKey::new(*mesh_id, ElementClass::Vertex),
            *self.mesh_id_to_vertex_slab(mesh_id)?,
        )
    }

    /// Returns the buffer and range within that buffer of the index data for
    /// the mesh with the given ID.
    ///
    /// If the mesh has no index data or wasn't allocated, returns None.
    pub fn mesh_index_slice(&self, mesh_id: &AssetId<Mesh>) -> Option<MeshBufferSlice<'_>> {
        self.slab_allocation_slice(
            &MeshAllocationKey::new(*mesh_id, ElementClass::Index),
            *self.mesh_id_to_index_slab(mesh_id)?,
        )
    }

    /// Returns the buffer and range within that buffer of the morph target data
    /// for the mesh with the given ID.
    ///
    /// If the mesh has no morph target data or wasn't allocated, returns None.
    #[cfg(feature = "morph")]
    pub fn mesh_morph_target_slice(&self, mesh_id: &AssetId<Mesh>) -> Option<MeshBufferSlice<'_>> {
        self.slab_allocation_slice(
            &MeshAllocationKey::new(*mesh_id, ElementClass::MorphTarget),
            *self.mesh_id_to_morph_target_slab(mesh_id)?,
        )
    }

    /// Returns the IDs of the vertex buffer and index buffer respectively for
    /// the mesh with the given ID.
    ///
    /// If the mesh wasn't allocated, or has no index data in the case of the
    /// index buffer, the corresponding element in the returned tuple will be
    /// None.
    pub fn mesh_slabs(&self, mesh_id: &AssetId<Mesh>) -> Option<MeshSlabs> {
        Some(MeshSlabs {
            vertex_slab_id: self.mesh_id_to_vertex_slab(mesh_id).cloned()?,
            index_slab_id: self.mesh_id_to_index_slab(mesh_id).cloned(),
            metadata_slab_id: self.mesh_id_to_metadata_slab(mesh_id).cloned(),
            #[cfg(feature = "morph")]
            morph_target_slab_id: self.mesh_id_to_morph_target_slab(mesh_id).cloned(),
        })
    }

    /// Returns the number of index allocations that this mesh allocator
    /// manages.
    pub fn index_allocation_count(&self) -> usize {
        self.key_to_slab
            .keys()
            .filter(|key| key.class == ElementClass::Index)
            .count()
    }

    /// Given the ID of a mesh, returns the ID of the slab that contains the
    /// metadata for that mesh, if it exists.
    fn mesh_id_to_metadata_slab(&self, mesh_id: &AssetId<Mesh>) -> Option<&SlabId<MeshSlabItem>> {
        self.key_to_slab
            .get(&MeshAllocationKey::new(*mesh_id, ElementClass::Metadata))
    }

    /// Given the ID of a mesh, returns the ID of the slab that contains the
    /// vertex data for that mesh, if it exists.
    fn mesh_id_to_vertex_slab(&self, mesh_id: &AssetId<Mesh>) -> Option<&SlabId<MeshSlabItem>> {
        self.key_to_slab
            .get(&MeshAllocationKey::new(*mesh_id, ElementClass::Vertex))
    }

    /// Given the ID of a mesh, returns the ID of the slab that contains the
    /// index data for that mesh, if it exists.
    fn mesh_id_to_index_slab(&self, mesh_id: &AssetId<Mesh>) -> Option<&SlabId<MeshSlabItem>> {
        self.key_to_slab
            .get(&MeshAllocationKey::new(*mesh_id, ElementClass::Index))
    }

    /// Given the ID of a mesh, returns the ID of the slab that contains the
    /// morph target data for that mesh, if it exists.
    #[cfg(feature = "morph")]
    fn mesh_id_to_morph_target_slab(
        &self,
        mesh_id: &AssetId<Mesh>,
    ) -> Option<&SlabId<MeshSlabItem>> {
        self.key_to_slab
            .get(&MeshAllocationKey::new(*mesh_id, ElementClass::MorphTarget))
    }

    /// Returns an iterator over all slabs that contain morph targets.
    #[cfg(feature = "morph")]
    pub fn morph_target_slabs(&self) -> impl Iterator<Item = MeshSlabId> {
        self.slabs.iter().filter_map(|(slab_id, slab)| {
            if matches!(slab.element_class(), ElementClass::MorphTarget) {
                Some(*slab_id)
            } else {
                None
            }
        })
    }

    pub fn metadata_slabs(&self) -> impl Iterator<Item = MeshSlabId> {
        self.slabs.iter().filter_map(|(slab_id, slab)| {
            if matches!(slab.element_class(), ElementClass::Metadata) {
                Some(*slab_id)
            } else {
                None
            }
        })
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
        let mut allocation_stage = self.slab_allocator.stage_allocation();

        // Loop over each mesh that was extracted this frame.
        for (mesh_id, mesh) in &extracted_meshes.extracted {
            let vertex_buffer_size = mesh.get_vertex_buffer_size() as u64;
            if vertex_buffer_size == 0 {
                warn!("Mesh {:?} contains no vertices.", mesh_id);
                continue;
            }

            // Allocate metadata.
            if mesh.final_aabb.is_some() || mesh.final_uv_ranges.iter().any(Option::is_some) {
                // If storage buffers are unsupported, we allocate uniform buffers for each mesh.
                if crate::storage_buffers_are_unsupported(&render_device.limits()) {
                    allocation_stage.allocate_large(
                        &MeshAllocationKey::new(*mesh_id, ElementClass::Metadata),
                        ElementLayout::metadata(false),
                    );
                } else {
                    allocation_stage.allocate(
                        &MeshAllocationKey::new(*mesh_id, ElementClass::Metadata),
                        size_of::<MeshMetadata>() as u64,
                        ElementLayout::metadata(true),
                        mesh_allocator_settings,
                    );
                }
            }

            // Allocate vertex data. Note that we can only pack mesh vertex data
            // together if the platform supports it.
            let vertex_element_layout = ElementLayout::vertex(mesh_vertex_buffer_layouts, mesh);
            if self.general_vertex_slabs_supported {
                allocation_stage.allocate(
                    &MeshAllocationKey::new(*mesh_id, ElementClass::Vertex),
                    vertex_buffer_size,
                    vertex_element_layout,
                    mesh_allocator_settings,
                );
            } else {
                allocation_stage.allocate_large(
                    &MeshAllocationKey::new(*mesh_id, ElementClass::Vertex),
                    vertex_element_layout,
                );
            }

            // Allocate index data.
            if let (Some(index_buffer_data), Some(index_element_layout)) =
                (mesh.get_index_buffer_bytes(), ElementLayout::index(mesh))
            {
                allocation_stage.allocate(
                    &MeshAllocationKey::new(*mesh_id, ElementClass::Index),
                    index_buffer_data.len() as u64,
                    index_element_layout,
                    mesh_allocator_settings,
                );
            }

            // Allocate morph target data.
            #[cfg(feature = "morph")]
            if let Some(morph_targets) = mesh.get_morph_targets() {
                use const_shader_layout::ShaderLayout;

                allocation_stage.allocate(
                    &MeshAllocationKey::new(*mesh_id, ElementClass::MorphTarget),
                    morph_targets.len() as u64 * MorphAttributes::SIZE.get(),
                    MORPH_ATTRIBUTE_ELEMENT_LAYOUT,
                    mesh_allocator_settings,
                );
            }
        }

        // Perform growth.
        allocation_stage.commit(render_device, render_queue);

        // Copy new mesh data in.
        for (mesh_id, mesh) in &extracted_meshes.extracted {
            let vertex_buffer_size = mesh.get_vertex_buffer_size() as u64;
            if vertex_buffer_size == 0 {
                continue;
            }
            self.copy_mesh_metadata(mesh_id, mesh, render_device, render_queue);
            self.copy_mesh_vertex_data(mesh_id, mesh, render_device, render_queue);
            self.copy_mesh_index_data(mesh_id, mesh, render_device, render_queue);
            #[cfg(feature = "morph")]
            self.copy_mesh_morph_target_data(mesh_id, mesh, render_device, render_queue);
        }
    }

    /// Copies vertex array data from a mesh into the appropriate spot in the
    /// slab.
    fn copy_mesh_metadata(
        &mut self,
        mesh_id: &AssetId<Mesh>,
        mesh: &Mesh,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) {
        const UV_RANGES_NONE: [Option<Aabb2d>; 2] = [None; 2];
        let metadata = match (mesh.final_aabb, mesh.final_uv_ranges) {
            (None, UV_RANGES_NONE) => return,
            _ => {
                let (aabb_center, aabb_half_extents) = mesh
                    .final_aabb
                    .map(|aabb| (aabb.center().into(), aabb.half_size().into()))
                    .unwrap_or_default();
                let uv_channels_min_and_extents = mesh.final_uv_ranges.map(|maybe_uv| {
                    maybe_uv
                        .map(|aabb2d| {
                            Vec4::new(
                                aabb2d.min.x,
                                aabb2d.min.y,
                                aabb2d.max.x - aabb2d.min.x,
                                aabb2d.max.y - aabb2d.min.y,
                            )
                        })
                        .unwrap_or(Vec4::new(0.0, 0.0, 1.0, 1.0))
                });
                MeshMetadata {
                    aabb_center,
                    aabb_half_extents,
                    uv_channels_min_and_extents,
                    ..Default::default()
                }
            }
        };
        // Call the generic function.
        self.copy_element_data(
            &MeshAllocationKey::new(*mesh_id, ElementClass::Metadata),
            size_of::<MeshMetadata>(),
            |mut slice| slice.copy_from_slice(bytemuck::cast_slice(&[metadata])),
            render_device,
            render_queue,
        );
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
        // Call the generic function.
        self.copy_element_data(
            &MeshAllocationKey::new(*mesh_id, ElementClass::Vertex),
            mesh.get_vertex_buffer_size(),
            |slice| mesh.write_packed_vertex_buffer_data(slice),
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
        let Some(index_data) = mesh.get_index_buffer_bytes() else {
            return;
        };

        // Call the generic function.
        self.copy_element_data(
            &MeshAllocationKey::new(*mesh_id, ElementClass::Index),
            index_data.len(),
            |mut slice| slice.copy_from_slice(index_data),
            render_device,
            render_queue,
        );
    }

    /// Copies morph target array data from a mesh into the appropriate spot in
    /// the slab.
    #[cfg(feature = "morph")]
    fn copy_mesh_morph_target_data(
        &mut self,
        mesh_id: &AssetId<Mesh>,
        mesh: &Mesh,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) {
        let Some(morph_targets) = mesh.get_morph_targets() else {
            return;
        };

        // Call the generic function.
        self.copy_element_data(
            &MeshAllocationKey::new(*mesh_id, ElementClass::MorphTarget),
            size_of_val(morph_targets),
            |mut slice| slice.copy_from_slice(bytemuck::cast_slice(morph_targets)),
            render_device,
            render_queue,
        );
    }

    /// Frees allocations for meshes that were removed, modified, or re-extracted
    /// this frame.
    fn free_meshes(&mut self, extracted_meshes: &ExtractedAssets<RenderMesh>) {
        let mut deallocation_stage = self.slab_allocator.stage_deallocation();

        // TODO: Consider explicitly reusing allocations for changed meshes of
        // the same size

        // Free every mesh that `allocate_meshes` is about to reallocate. Despite
        // its name, `added` holds every mesh extracted this frame rather than only
        // the new ones, so it's exactly that set. This catches a mesh
        // removed from `Assets` and reinserted under the same ID, which arrives as
        // `Removed` then `Added` and so never appears in `modified`.
        let meshes_to_free = extracted_meshes
            .removed
            .iter()
            .chain(extracted_meshes.modified.iter())
            .chain(extracted_meshes.added.iter());

        for mesh_id in meshes_to_free {
            deallocation_stage.free(&MeshAllocationKey::new(*mesh_id, ElementClass::Metadata));
            deallocation_stage.free(&MeshAllocationKey::new(*mesh_id, ElementClass::Vertex));
            deallocation_stage.free(&MeshAllocationKey::new(*mesh_id, ElementClass::Index));
            #[cfg(feature = "morph")]
            deallocation_stage.free(&MeshAllocationKey::new(*mesh_id, ElementClass::MorphTarget));
        }

        deallocation_stage.commit();
    }
}

impl ElementLayout {
    /// Creates an [`ElementLayout`] for mesh data of the given class (vertex or
    /// index) with the given byte size.
    fn new(class: ElementClass, size: u64) -> ElementLayout {
        const {
            assert!(4 == COPY_BUFFER_ALIGNMENT);
        }

        // Use `ElementLayout::metadata` instead, as it needs to specify whether to use storage buffer.
        assert!(class != ElementClass::Metadata);

        // this is equivalent to `4 / gcd(4,size)` but lets us not implement gcd.
        // ping @atlv if above assert ever fails (likely never)
        let elements_per_slot = [1, 4, 2, 4][size as usize & 3];
        ElementLayout {
            class,
            size,
            // Make sure that slot boundaries begin and end on
            // `COPY_BUFFER_ALIGNMENT`-byte (4-byte) boundaries.
            elements_per_slot,
            buffer_usages: class.buffer_usages(true),
        }
    }

    fn metadata(storage_buffers_are_usable: bool) -> ElementLayout {
        ElementLayout {
            class: ElementClass::Metadata,
            size: size_of::<MeshMetadata>() as u64,
            elements_per_slot: 1,
            buffer_usages: ElementClass::Metadata.buffer_usages(storage_buffers_are_usable),
        }
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

impl SlabItemLayout for ElementLayout {
    fn size(&self) -> u64 {
        self.size
    }

    fn elements_per_slot(&self) -> u32 {
        self.elements_per_slot
    }

    fn buffer_usages(&self) -> BufferUsages {
        self.buffer_usages
    }
}

impl ElementClass {
    /// Returns the `wgpu` [`BufferUsages`] appropriate for a buffer of this
    /// class.
    const fn buffer_usages(&self, metadata_use_storage_buffer: bool) -> BufferUsages {
        match *self {
            ElementClass::Metadata => {
                if metadata_use_storage_buffer {
                    BufferUsages::STORAGE
                } else {
                    BufferUsages::UNIFORM
                }
            }
            ElementClass::Vertex => BufferUsages::VERTEX,
            ElementClass::Index => BufferUsages::INDEX,
            #[cfg(feature = "morph")]
            ElementClass::MorphTarget => BufferUsages::STORAGE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_dummy_device;
    use bevy_asset::{uuid::Uuid, RenderAssetUsages};
    use bevy_math::bounding::Aabb3d;
    use bevy_mesh::PrimitiveTopology;
    use glam::{Vec2, Vec3};

    fn test_mesh() -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0f32, 0.0, 0.0]; 64]);
        mesh
    }

    /// A mesh that exercises every [`ElementClass`] at once.
    fn full_mesh() -> Mesh {
        let mut mesh = test_mesh();
        mesh.insert_indices(Indices::U32((0..64).collect()));
        mesh.final_aabb = Some(Aabb3d::new(Vec3::ZERO, Vec3::ONE));
        mesh.final_uv_ranges[0] = Some(Aabb2d::new(Vec2::ZERO, Vec2::ONE));
        #[cfg(feature = "morph")]
        mesh.set_morph_targets(vec![MorphAttributes::default(); 64]);
        mesh
    }

    /// A mesh whose vertex layout differs from [`test_mesh`], so that it sorts
    /// into a different general slab.
    fn wider_vertex_mesh() -> Mesh {
        let mut mesh = test_mesh();
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 0.0, 1.0]; 64]);
        mesh
    }

    /// Builds a [`MeshAllocator`] backed by an empty slab allocator.
    ///
    /// Clearing `general_vertex_slabs_supported` sends every vertex array into a
    /// slab of its own.
    fn mesh_allocator(general_vertex_slabs_supported: bool) -> MeshAllocator {
        MeshAllocator {
            slab_allocator: SlabAllocator::new(),
            general_vertex_slabs_supported,
        }
    }

    /// Allocator tuning with a small large-object threshold, so that an
    /// otherwise modest test mesh is big enough to demand a slab of its own.
    fn small_slab_settings() -> MeshAllocatorSettings {
        MeshAllocatorSettings {
            slab_allocator_settings: SlabAllocatorSettings {
                min_slab_size: 1024,
                max_slab_size: 4096,
                large_threshold: 512,
                growth_factor: 1.5,
            },
            extra_buffer_usages: BufferUsages::empty(),
        }
    }

    fn mesh_id(id: u128) -> AssetId<Mesh> {
        AssetId::<Mesh>::Uuid {
            uuid: Uuid::from_u128(id),
        }
    }

    /// Whether the allocator currently holds an allocation of the given class
    /// for the given mesh.
    fn has_allocation(
        mesh_allocator: &MeshAllocator,
        mesh_id: AssetId<Mesh>,
        class: ElementClass,
    ) -> bool {
        mesh_allocator
            .key_to_slab
            .contains_key(&MeshAllocationKey::new(mesh_id, class))
    }

    /// Builds the extraction output for a mesh that was extracted this frame.
    ///
    /// This mirrors what `extract_render_asset` produces for an
    /// `AssetEvent::Added`: the mesh lands in `extracted` and `added`, and in
    /// neither `removed` nor `modified`.
    fn extracted_mesh(id: AssetId<Mesh>, mesh: Mesh) -> ExtractedAssets<RenderMesh> {
        let mut extracted_meshes = ExtractedAssets::<RenderMesh>::default();
        extracted_meshes.extracted.push((id, mesh));
        extracted_meshes.added.insert(id);
        extracted_meshes
    }

    /// Builds the extraction output for a mesh modified in place this frame.
    ///
    /// This mirrors what `extract_render_asset` produces for an
    /// `AssetEvent::Modified`, as caused by `Assets::get_mut`: the mesh is
    /// re-extracted, so it lands in `modified` on top of `extracted` and `added`.
    fn modified_mesh(id: AssetId<Mesh>, mesh: Mesh) -> ExtractedAssets<RenderMesh> {
        let mut extracted_meshes = extracted_mesh(id, mesh);
        extracted_meshes.modified.insert(id);
        extracted_meshes
    }

    /// `free_meshes` must release meshes that are merely being re-extracted, not
    /// only those flagged `removed` or `modified`.
    #[test]
    fn free_meshes_releases_reextracted_meshes() {
        let (render_device, render_queue) = create_dummy_device();
        let settings = MeshAllocatorSettings::default();
        let mut mesh_vertex_buffer_layouts = MeshVertexBufferLayouts::default();
        let mut mesh_allocator = mesh_allocator(true);

        let mesh_id = mesh_id(1);
        let extracted_meshes = extracted_mesh(mesh_id, test_mesh());

        mesh_allocator.allocate_meshes(
            &settings,
            &extracted_meshes,
            &mut mesh_vertex_buffer_layouts,
            &render_device,
            &render_queue,
        );
        assert!(mesh_allocator.mesh_vertex_slice(&mesh_id).is_some());

        // Being present in `added` alone must be enough to release the previous
        // allocation.
        mesh_allocator.free_meshes(&extracted_meshes);

        assert!(
            mesh_allocator.key_to_slab.is_empty(),
            "a re-extracted mesh was not freed, so its old allocation would leak"
        );
        assert_eq!(mesh_allocator.slab_count(), 0);
    }

    /// A mesh flagged `modified` must be freed even when it isn't re-extracted.
    ///
    /// `added` covers meshes that come back around for reallocation, but a mesh
    /// can be modified and then leave `Assets` without emitting `Unused`, in
    /// which case `modified` is the only record we get of it.
    #[test]
    fn free_meshes_releases_modified_meshes_that_were_not_reextracted() {
        let (render_device, render_queue) = create_dummy_device();
        let settings = MeshAllocatorSettings::default();
        let mut mesh_vertex_buffer_layouts = MeshVertexBufferLayouts::default();
        let mut mesh_allocator = mesh_allocator(true);

        let mesh_id = mesh_id(1);
        mesh_allocator.allocate_meshes(
            &settings,
            &extracted_mesh(mesh_id, test_mesh()),
            &mut mesh_vertex_buffer_layouts,
            &render_device,
            &render_queue,
        );
        assert!(mesh_allocator.mesh_vertex_slice(&mesh_id).is_some());

        let mut extracted_meshes = ExtractedAssets::<RenderMesh>::default();
        extracted_meshes.modified.insert(mesh_id);
        mesh_allocator.free_meshes(&extracted_meshes);

        assert!(
            mesh_allocator.key_to_slab.is_empty(),
            "a modified mesh that wasn't re-extracted was not freed"
        );
        assert_eq!(mesh_allocator.slab_count(), 0);
    }

    /// `free_meshes` must release every class of allocation a mesh holds, not
    /// just its vertex data.
    #[test]
    fn free_meshes_releases_every_element_class() {
        let (render_device, render_queue) = create_dummy_device();
        let settings = MeshAllocatorSettings::default();
        let mut mesh_vertex_buffer_layouts = MeshVertexBufferLayouts::default();
        let mut mesh_allocator = mesh_allocator(true);

        let mesh_id = mesh_id(1);
        let extracted_meshes = extracted_mesh(mesh_id, full_mesh());
        mesh_allocator.allocate_meshes(
            &settings,
            &extracted_meshes,
            &mut mesh_vertex_buffer_layouts,
            &render_device,
            &render_queue,
        );

        assert!(has_allocation(
            &mesh_allocator,
            mesh_id,
            ElementClass::Vertex
        ));
        assert!(has_allocation(
            &mesh_allocator,
            mesh_id,
            ElementClass::Index
        ));
        assert!(has_allocation(
            &mesh_allocator,
            mesh_id,
            ElementClass::Metadata
        ));
        #[cfg(feature = "morph")]
        assert!(has_allocation(
            &mesh_allocator,
            mesh_id,
            ElementClass::MorphTarget
        ));

        mesh_allocator.free_meshes(&extracted_meshes);

        assert!(
            mesh_allocator.key_to_slab.is_empty(),
            "at least one element class was left allocated, so it would leak"
        );
        assert_eq!(mesh_allocator.slab_count(), 0);
    }

    /// A mesh that loses part of its data on re-extraction must give up the
    /// matching allocations, which nothing will reallocate.
    #[test]
    fn reextracting_a_mesh_that_drops_its_extra_data_frees_those_allocations() {
        let (render_device, render_queue) = create_dummy_device();
        let settings = MeshAllocatorSettings::default();
        let mut mesh_vertex_buffer_layouts = MeshVertexBufferLayouts::default();
        let mut mesh_allocator = mesh_allocator(true);

        let mesh_id = mesh_id(1);
        mesh_allocator.allocate_meshes(
            &settings,
            &extracted_mesh(mesh_id, full_mesh()),
            &mut mesh_vertex_buffer_layouts,
            &render_device,
            &render_queue,
        );
        assert!(has_allocation(
            &mesh_allocator,
            mesh_id,
            ElementClass::Index
        ));

        // The same ID comes back as a bare vertex-only mesh.
        let extracted_meshes = extracted_mesh(mesh_id, test_mesh());
        mesh_allocator.free_meshes(&extracted_meshes);
        mesh_allocator.allocate_meshes(
            &settings,
            &extracted_meshes,
            &mut mesh_vertex_buffer_layouts,
            &render_device,
            &render_queue,
        );

        assert!(has_allocation(
            &mesh_allocator,
            mesh_id,
            ElementClass::Vertex
        ));
        assert!(
            !has_allocation(&mesh_allocator, mesh_id, ElementClass::Index),
            "index data was dropped by the mesh but its allocation survived"
        );
        assert!(
            !has_allocation(&mesh_allocator, mesh_id, ElementClass::Metadata),
            "metadata was dropped by the mesh but its allocation survived"
        );
        #[cfg(feature = "morph")]
        assert!(
            !has_allocation(&mesh_allocator, mesh_id, ElementClass::MorphTarget),
            "morph targets were dropped by the mesh but their allocation survived"
        );
    }

    /// Changing a mesh's vertex layout moves it to a different slab, and the
    /// slab it leaves behind must be reclaimed.
    #[test]
    fn reextracting_a_mesh_with_a_new_vertex_layout_reclaims_the_old_slab() {
        let (render_device, render_queue) = create_dummy_device();
        let settings = MeshAllocatorSettings::default();
        let mut mesh_vertex_buffer_layouts = MeshVertexBufferLayouts::default();
        let mut mesh_allocator = mesh_allocator(true);

        let mesh_id = mesh_id(1);
        mesh_allocator.allocate_meshes(
            &settings,
            &extracted_mesh(mesh_id, test_mesh()),
            &mut mesh_vertex_buffer_layouts,
            &render_device,
            &render_queue,
        );
        assert_eq!(mesh_allocator.slab_count(), 1);
        let original_slab =
            mesh_allocator.key_to_slab[&MeshAllocationKey::new(mesh_id, ElementClass::Vertex)];

        // Adding a normal attribute widens the vertex, which needs a slab with a
        // different element layout.
        let extracted_meshes = extracted_mesh(mesh_id, wider_vertex_mesh());
        mesh_allocator.free_meshes(&extracted_meshes);
        mesh_allocator.allocate_meshes(
            &settings,
            &extracted_meshes,
            &mut mesh_vertex_buffer_layouts,
            &render_device,
            &render_queue,
        );

        let new_slab =
            mesh_allocator.key_to_slab[&MeshAllocationKey::new(mesh_id, ElementClass::Vertex)];
        assert_ne!(
            new_slab, original_slab,
            "the wider vertex should have landed in a slab with a different layout"
        );
        assert_eq!(
            mesh_allocator.slab_count(),
            1,
            "the slab the mesh moved out of was not reclaimed"
        );
        assert!(mesh_allocator.mesh_vertex_slice(&mesh_id).is_some());
    }

    /// One frame-loop scenario for [`assert_steady_state`].
    struct SteadyStateCase {
        /// Whether the mesh arrives as merely re-extracted or as modified.
        build: fn(AssetId<Mesh>, Mesh) -> ExtractedAssets<RenderMesh>,
        build_mesh: fn() -> Mesh,
        settings: MeshAllocatorSettings,
        general_vertex_slabs_supported: bool,
    }

    impl Default for SteadyStateCase {
        fn default() -> Self {
            Self {
                build: extracted_mesh,
                build_mesh: test_mesh,
                settings: MeshAllocatorSettings::default(),
                general_vertex_slabs_supported: true,
            }
        }
    }

    /// Runs `rounds` frames of free-then-allocate over a single mesh ID and
    /// asserts that slab memory reaches a steady state.
    fn assert_steady_state(case: SteadyStateCase, rounds: usize) {
        let SteadyStateCase {
            build,
            build_mesh,
            settings,
            general_vertex_slabs_supported,
        } = case;

        let (render_device, render_queue) = create_dummy_device();
        let mut mesh_vertex_buffer_layouts = MeshVertexBufferLayouts::default();
        let mut mesh_allocator = mesh_allocator(general_vertex_slabs_supported);

        let mesh_id = mesh_id(1);

        let mut baseline = None;
        for _ in 0..rounds {
            let extracted_meshes = build(mesh_id, build_mesh());
            mesh_allocator.free_meshes(&extracted_meshes);
            mesh_allocator.allocate_meshes(
                &settings,
                &extracted_meshes,
                &mut mesh_vertex_buffer_layouts,
                &render_device,
                &render_queue,
            );

            let size = mesh_allocator.slabs_size();
            let slab_count = mesh_allocator.slab_count();
            match baseline {
                None => baseline = Some((size, slab_count)),
                Some(baseline) => {
                    assert_eq!(
                        (size, slab_count),
                        baseline,
                        "slab memory grew across frames"
                    );
                }
            }
        }

        assert!(mesh_allocator.mesh_vertex_slice(&mesh_id).is_some());
    }

    /// Re-extracting the same mesh ID every frame must reach a steady state.
    #[test]
    fn reextracting_the_same_mesh_does_not_grow_the_slabs() {
        assert_steady_state(SteadyStateCase::default(), 32);
    }

    /// Modifying the same mesh in place every frame must reach a steady state.
    #[test]
    fn modifying_a_mesh_in_place_does_not_grow_the_slabs() {
        assert_steady_state(
            SteadyStateCase {
                build: modified_mesh,
                ..SteadyStateCase::default()
            },
            32,
        );
    }

    /// A mesh carrying every element class must reach a steady state too, so
    /// that a leak confined to one class cannot hide behind the vertex data.
    #[test]
    fn reextracting_a_mesh_with_every_element_class_does_not_grow_the_slabs() {
        assert_steady_state(
            SteadyStateCase {
                build_mesh: full_mesh,
                ..SteadyStateCase::default()
            },
            32,
        );
    }

    /// Data too big to share a general slab gets one of its own, so a missed
    /// free leaks a whole slab rather than a slot inside one.
    #[test]
    fn reextracting_a_mesh_too_large_for_a_general_slab_does_not_grow_the_slabs() {
        assert_steady_state(
            SteadyStateCase {
                build_mesh: full_mesh,
                settings: small_slab_settings(),
                ..SteadyStateCase::default()
            },
            32,
        );
    }

    /// The other route to a dedicated slab, taken when the platform cannot
    /// share vertex slabs at all.
    #[test]
    fn reextracting_a_mesh_without_general_vertex_slabs_does_not_grow_the_slabs() {
        assert_steady_state(
            SteadyStateCase {
                build_mesh: full_mesh,
                general_vertex_slabs_supported: false,
                ..SteadyStateCase::default()
            },
            32,
        );
    }
}
