#define_import_path bevy_pbr::mesh_bindings

#import bevy_pbr::mesh_types::Mesh

#ifndef MESHLET_MESH_MATERIAL_PASS
#ifdef PER_OBJECT_BUFFER_BATCH_SIZE
@group(2) @binding(0) var<uniform> mesh: array<Mesh, #{PER_OBJECT_BUFFER_BATCH_SIZE}u>;
#else
@group(2) @binding(0) var<storage> mesh: array<Mesh>;
#endif // PER_OBJECT_BUFFER_BATCH_SIZE
#endif  // MESHLET_MESH_MATERIAL_PASS
