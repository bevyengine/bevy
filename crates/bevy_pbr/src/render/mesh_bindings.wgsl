#define_import_path bevy_pbr::mesh_bindings

#import bevy_pbr::mesh_types Mesh

#ifdef MESH_BINDGROUP_1

@group(1) @binding(0)
#ifdef MESH_BATCH_SIZE
var<uniform> mesh: array<Mesh, #{MESH_BATCH_SIZE}u>;
#else
var<storage> mesh: array<Mesh>;
#endif // MESH_BATCH_SIZE

#else // MESH_BINDGROUP_1

@group(2) @binding(0)
#ifdef MESH_BATCH_SIZE
var<uniform> mesh: array<Mesh, #{MESH_BATCH_SIZE}u>;
#else
var<storage> mesh: array<Mesh>;
#endif // MESH_BATCH_SIZE

#endif // MESH_BINDGROUP_1
