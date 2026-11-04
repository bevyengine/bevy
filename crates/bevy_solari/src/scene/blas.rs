use alloc::collections::VecDeque;
use bevy_asset::AssetId;
use bevy_ecs::{
    resource::Resource,
    system::{Res, ResMut},
};
use bevy_mesh::{Indices, Mesh, MeshRaytracingFlags};
use bevy_platform::collections::HashMap;
use bevy_render::{
    diagnostic::{DiagnosticsRecorder, RecordDiagnostics},
    mesh::{
        allocator::{MeshAllocator, MeshBufferSlice},
        RenderMesh,
    },
    render_asset::ExtractedAssets,
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
};

/// After compacting this many vertices worth of meshes per frame, no further BLAS will be compacted.
/// Lower this number to distribute the work across more frames.
const MAX_COMPACTION_VERTICES_PER_FRAME: u32 = 400_000;

/// Under the `wgpu_hal` build path, we need to manage BLAS lifetimes ourselves.
/// Since solari keeps both current and previous frame TLAS's around, only after
/// two TLAS builds since we marked it for deletion is it safe to delete a BLAS.
const TLAS_BUILDS_BEFORE_DELETION_ALLOWED: usize = 2;

#[derive(Resource, Default)]
pub struct BlasManager {
    blas: HashMap<AssetId<Mesh>, MeshBlas>,
    compaction_queue: VecDeque<(BlasKey, u32, bool)>,
    changed: Vec<AssetId<Mesh>>,
    /// BLAS that are pending deletion, one batch per TLAS build. The back batch collects
    /// retirements since the last build, and every batch ahead of it has one more build to wait
    /// out.
    pending_deletions: VecDeque<Vec<Blas>>,
}

impl BlasManager {
    pub fn get(&self, key: &BlasKey) -> Option<&Blas> {
        self.blas.get(&key.mesh)?.get(key.opacity)
    }

    pub fn device_address(&self, key: &BlasKey) -> Option<u64> {
        self.get(key)?.handle()
    }

    /// If a mesh is raytracing compatible, but its [`Mesh::raytracing`] flags don't declare
    /// the required opacity.
    pub fn is_undeclared(&self, key: &BlasKey) -> bool {
        self.blas
            .get(&key.mesh)
            .is_some_and(|mesh| !mesh.requires.contains(key.opacity.flag()))
    }

    fn require(&mut self, mesh: AssetId<Mesh>, flags: MeshRaytracingFlags) {
        self.blas.entry(mesh).or_default().requires = flags;
    }

    pub fn changed_meshes(&self) -> &[AssetId<Mesh>] {
        &self.changed
    }

    pub fn note_tlas_build(&mut self) {
        if !self.pending_deletions.is_empty() {
            self.pending_deletions.push_back(Vec::new());
        }
    }

    fn insert(&mut self, key: BlasKey, blas: Blas) {
        let slot = self.blas.entry(key.mesh).or_default().slot_mut(key.opacity);
        if let Some(old) = slot.replace(blas) {
            self.retire(old);
        }

        self.changed.push(key.mesh);
    }

    fn remove_mesh(&mut self, mesh: AssetId<Mesh>) {
        self.changed.push(mesh);
        self.compaction_queue.retain(|(key, ..)| key.mesh != mesh);

        if let Some(removed) = self.blas.remove(&mesh) {
            for blas in removed.into_iter() {
                self.retire(blas);
            }
        }
    }

    fn retire(&mut self, blas: Blas) {
        match self.pending_deletions.back_mut() {
            Some(batch) => batch.push(blas),
            None => self.pending_deletions.push_back(vec![blas]),
        }
    }
}

struct MeshBlas {
    requires: MeshRaytracingFlags,
    opaque: Option<Blas>,
    non_opaque: Option<Blas>,
}

impl Default for MeshBlas {
    fn default() -> Self {
        Self {
            requires: MeshRaytracingFlags::empty(),
            opaque: None,
            non_opaque: None,
        }
    }
}

impl MeshBlas {
    fn get(&self, opacity: BlasOpacity) -> Option<&Blas> {
        match opacity {
            BlasOpacity::Opaque => self.opaque.as_ref(),
            BlasOpacity::NonOpaque => self.non_opaque.as_ref(),
        }
    }

    fn slot_mut(&mut self, opacity: BlasOpacity) -> &mut Option<Blas> {
        match opacity {
            BlasOpacity::Opaque => &mut self.opaque,
            BlasOpacity::NonOpaque => &mut self.non_opaque,
        }
    }

    fn into_iter(self) -> impl Iterator<Item = Blas> {
        self.opaque.into_iter().chain(self.non_opaque)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BlasKey {
    pub mesh: AssetId<Mesh>,
    pub opacity: BlasOpacity,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum BlasOpacity {
    Opaque,
    NonOpaque,
}

impl BlasOpacity {
    fn declared(flags: MeshRaytracingFlags) -> impl Iterator<Item = Self> {
        [Self::Opaque, Self::NonOpaque]
            .into_iter()
            .filter(move |opacity: &BlasOpacity| flags.contains(opacity.flag()))
    }

    pub fn flag(self) -> MeshRaytracingFlags {
        match self {
            Self::Opaque => MeshRaytracingFlags::OPAQUE,
            Self::NonOpaque => MeshRaytracingFlags::NON_OPAQUE,
        }
    }

    fn geometry_flags(self) -> AccelerationStructureGeometryFlags {
        match self {
            Self::Opaque => AccelerationStructureGeometryFlags::OPAQUE,
            Self::NonOpaque => AccelerationStructureGeometryFlags::empty(),
        }
    }
}

pub fn prepare_raytracing_blas(
    mut blas_manager: ResMut<BlasManager>,
    extracted_meshes: Res<ExtractedAssets<RenderMesh>>,
    mesh_allocator: Res<MeshAllocator>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut diagnostics: Option<ResMut<DiagnosticsRecorder>>,
) {
    blas_manager.changed.clear();

    // Delete BLAS for deleted or modified meshes
    for asset_id in extracted_meshes
        .removed
        .iter()
        .chain(extracted_meshes.modified.iter())
    {
        blas_manager.remove_mesh(*asset_id);
    }

    if extracted_meshes.extracted.is_empty() {
        return;
    }

    // Record which BLAS added or changed meshes declare, even if none
    for (asset_id, mesh) in &extracted_meshes.extracted {
        if is_mesh_raytracing_compatible(mesh) {
            blas_manager.require(*asset_id, mesh.raytracing);
        }
    }

    // Create a BLAS for each opacity that added or changed meshes declare
    let blas_resources = extracted_meshes
        .extracted
        .iter()
        .filter(|(_, mesh)| is_mesh_raytracing_compatible(mesh))
        .flat_map(|(asset_id, mesh)| {
            BlasOpacity::declared(mesh.raytracing).map(|opacity| BlasKey {
                mesh: *asset_id,
                opacity,
            })
        })
        .map(|key| {
            let vertex_slice = mesh_allocator.mesh_vertex_slice(&key.mesh).unwrap();
            let index_slice = mesh_allocator.mesh_index_slice(&key.mesh).unwrap();

            let (blas, blas_size) = allocate_blas(&vertex_slice, &index_slice, key, &render_device);

            blas_manager.insert(key, blas);
            blas_manager
                .compaction_queue
                .push_back((key, blas_size.vertex_count, false));

            (key, vertex_slice, index_slice, blas_size)
        })
        .collect::<Vec<_>>();

    // Build geometry into each BLAS
    let build_entries = blas_resources
        .iter()
        .map(|(key, vertex_slice, index_slice, blas_size)| {
            let geometry = BlasTriangleGeometry {
                size: blas_size,
                vertex_buffer: vertex_slice.buffer,
                first_vertex: vertex_slice.range.start,
                vertex_stride: 48,
                index_buffer: Some(index_slice.buffer),
                first_index: Some(index_slice.range.start),
                transform_buffer: None,
                transform_buffer_offset: None,
            };
            BlasBuildEntry {
                blas: blas_manager.get(key).unwrap(),
                geometry: BlasGeometries::TriangleGeometries(vec![geometry]),
            }
        })
        .collect::<Vec<_>>();

    let mut command_encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("blas_build_command_encoder"),
    });
    let time_span = diagnostics
        .as_mut()
        .map(|diagnostics| diagnostics.time_span(&mut command_encoder, "blas_build"));
    command_encoder.build_acceleration_structures(&build_entries, &[]);
    if let Some(time_span) = time_span {
        time_span.end(&mut command_encoder);
    }
    render_queue.submit([command_encoder.finish()]);
}

pub fn compact_raytracing_blas(
    mut blas_manager: ResMut<BlasManager>,
    render_queue: Res<RenderQueue>,
) {
    let queue_size = blas_manager.compaction_queue.len();
    let mut meshes_processed = 0;
    let mut vertices_compacted = 0;

    while !blas_manager.compaction_queue.is_empty()
        && vertices_compacted < MAX_COMPACTION_VERTICES_PER_FRAME
        && meshes_processed < queue_size
    {
        meshes_processed += 1;

        let (key, vertex_count, compaction_started) =
            blas_manager.compaction_queue.pop_front().unwrap();

        let Some(blas) = blas_manager.get(&key) else {
            continue;
        };

        if !compaction_started {
            blas.prepare_compaction_async(|_| {});
        }

        if blas.ready_for_compaction() {
            let compacted_blas = render_queue.compact_blas(blas);
            blas_manager.insert(key, compacted_blas);

            vertices_compacted += vertex_count;
            continue;
        }

        // BLAS not ready for compaction, put back in queue
        blas_manager
            .compaction_queue
            .push_back((key, vertex_count, true));
    }
}

pub fn delete_raytracing_blas(
    mut blas_manager: ResMut<BlasManager>,
    render_queue: Res<RenderQueue>,
) {
    if blas_manager.pending_deletions.len() <= TLAS_BUILDS_BEFORE_DELETION_ALLOWED {
        return;
    }

    if let Some(deletable) = blas_manager
        .pending_deletions
        .pop_front()
        .filter(|b| !b.is_empty())
    {
        render_queue.on_submitted_work_done(move || drop(deletable));
    }
}

fn allocate_blas(
    vertex_slice: &MeshBufferSlice,
    index_slice: &MeshBufferSlice,
    key: BlasKey,
    render_device: &RenderDevice,
) -> (Blas, BlasTriangleGeometrySizeDescriptor) {
    let blas_size = BlasTriangleGeometrySizeDescriptor {
        vertex_format: Mesh::ATTRIBUTE_POSITION.format,
        vertex_count: vertex_slice.range.len() as u32,
        index_format: Some(IndexFormat::Uint32),
        index_count: Some(index_slice.range.len() as u32),
        flags: key.opacity.geometry_flags(),
    };

    // TODO: If we ever introduce BLAS refits, we need to be aware of the TLAS double-buffer
    // to avoid invalidating the previous frame TLAS
    let blas = render_device.wgpu_device().create_blas(
        &CreateBlasDescriptor {
            label: Some(&format!("{} {:?}", key.mesh, key.opacity)),
            flags: AccelerationStructureFlags::PREFER_FAST_TRACE
                | AccelerationStructureFlags::ALLOW_COMPACTION,
            update_mode: AccelerationStructureUpdateMode::Build,
        },
        BlasGeometrySizeDescriptors::Triangles {
            descriptors: vec![blas_size.clone()],
        },
    );

    (blas, blas_size)
}

fn is_mesh_raytracing_compatible(mesh: &Mesh) -> bool {
    let triangle_list = mesh.primitive_topology() == PrimitiveTopology::TriangleList;
    let vertex_attributes = mesh
        .attributes()
        .map(|(attribute, _)| (attribute.id, attribute.format))
        .eq([
            (Mesh::ATTRIBUTE_POSITION.id, Mesh::ATTRIBUTE_POSITION.format),
            (Mesh::ATTRIBUTE_NORMAL.id, Mesh::ATTRIBUTE_NORMAL.format),
            (Mesh::ATTRIBUTE_UV_0.id, Mesh::ATTRIBUTE_UV_0.format),
            (Mesh::ATTRIBUTE_TANGENT.id, Mesh::ATTRIBUTE_TANGENT.format),
        ]);
    let indexed_32 = matches!(mesh.indices(), Some(Indices::U32(..)));
    triangle_list && vertex_attributes && indexed_32
}
