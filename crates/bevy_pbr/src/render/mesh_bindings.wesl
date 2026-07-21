import bevy_pbr::render::mesh_types::Mesh;
import bevy_render::mesh::metadata_types::MeshMetadata;

@if(!MESHLET_MESH_MATERIAL_PASS && PER_OBJECT_BUFFER_BATCH_SIZE)
@group(2) @binding(0) var<uniform> mesh: array<Mesh, u32(constants::PER_OBJECT_BUFFER_BATCH_SIZE)>;
@if(!MESHLET_MESH_MATERIAL_PASS && !PER_OBJECT_BUFFER_BATCH_SIZE)
@group(2) @binding(0) var<storage> mesh: array<Mesh>;

@if(!MESHLET_MESH_MATERIAL_PASS && METADATA_USE_UNIFORM_BUFFERS)
@group(2) @binding(9) var<uniform> metadata: MeshMetadata;
@if(!MESHLET_MESH_MATERIAL_PASS && !METADATA_USE_UNIFORM_BUFFERS)
@group(2) @binding(9) var<storage> metadata: array<MeshMetadata>;
