//! Manages mesh vertex and index buffers.

use alloc::borrow::Cow;
use bevy_mesh::Indices;

use bevy_app::{App, Plugin};
use bevy_asset::AssetId;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    resource::Resource,
    schedule::IntoScheduleConfigs as _,
    system::{Res, ResMut},
    world::{FromWorld, World},
};
use wgpu::{BufferUsages, DownlevelFlags, COPY_BUFFER_ALIGNMENT};

#[cfg(feature = "morph")]
use bevy_mesh::morph::MorphAttributes;

use crate::{
    mesh::{Mesh, MeshVertexBufferLayouts, RenderMesh},
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
static MORPH_ATTRIBUTE_ELEMENT_LAYOUT: ElementLayout = ElementLayout {
    class: ElementClass::MorphTarget,
    size: size_of::<MorphAttributes>() as u64,
    elements_per_slot: 1,
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
    /// The slab storing the mesh's morph target displacements, if the mesh has
    /// morph targets.
    #[cfg(feature = "morph")]
    pub morph_target_slab_id: Option<MeshSlabId>,
}

impl Slab<MeshSlabItem> {
    /// Returns the type of buffer that this is: vertex, index, or morph target.
    #[cfg(feature = "morph")]
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
    /// Data for a vertex.
    Vertex,
    /// A vertex index.
    Index,
    #[cfg(feature = "morph")]
    /// Displacement data for a morph target.
    MorphTarget,
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
pub struct ElementLayout {
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
                continue;
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
                allocation_stage.allocate(
                    &MeshAllocationKey::new(*mesh_id, ElementClass::MorphTarget),
                    morph_targets.len() as u64 * size_of::<MorphAttributes>() as u64,
                    MORPH_ATTRIBUTE_ELEMENT_LAYOUT,
                    mesh_allocator_settings,
                );
            }
        }

        // Perform growth.
        allocation_stage.commit(render_device, render_queue);

        // Copy new mesh data in.
        for (mesh_id, mesh) in &extracted_meshes.extracted {
            self.copy_mesh_vertex_data(mesh_id, mesh, render_device, render_queue);
            self.copy_mesh_index_data(mesh_id, mesh, render_device, render_queue);
            #[cfg(feature = "morph")]
            self.copy_mesh_morph_target_data(mesh_id, mesh, render_device, render_queue);
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

    /// Frees allocations for meshes that were removed or modified this frame.
    fn free_meshes(&mut self, extracted_meshes: &ExtractedAssets<RenderMesh>) {
        let mut deallocation_stage = self.slab_allocator.stage_deallocation();

        // TODO: Consider explicitly reusing allocations for changed meshes of
        // the same size
        let meshes_to_free = extracted_meshes
            .removed
            .iter()
            .chain(extracted_meshes.modified.iter());

        for mesh_id in meshes_to_free {
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
        // this is equivalent to `4 / gcd(4,size)` but lets us not implement gcd.
        // ping @atlv if above assert ever fails (likely never)
        let elements_per_slot = [1, 4, 2, 4][size as usize & 3];
        ElementLayout {
            class,
            size,
            // Make sure that slot boundaries begin and end on
            // `COPY_BUFFER_ALIGNMENT`-byte (4-byte) boundaries.
            elements_per_slot,
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
        self.class.buffer_usages()
    }
}

impl ElementClass {
    /// Returns the `wgpu` [`BufferUsages`] appropriate for a buffer of this
    /// class.
    fn buffer_usages(&self) -> BufferUsages {
        match *self {
            ElementClass::Vertex => BufferUsages::VERTEX,
            ElementClass::Index => BufferUsages::INDEX,
            #[cfg(feature = "morph")]
            ElementClass::MorphTarget => BufferUsages::STORAGE,
        }
    }
}
