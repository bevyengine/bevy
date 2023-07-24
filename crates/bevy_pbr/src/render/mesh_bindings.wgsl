#define_import_path bevy_pbr::mesh_bindings

#import bevy_pbr::mesh_types Mesh

#ifdef MESH_BINDGROUP_1

#ifdef MESH_BATCH_SIZE
@group(1) @binding(0)
var<uniform> mesh: array<Mesh, #{MESH_BATCH_SIZE}u>;
#else
@group(1) @binding(0)
var<storage> mesh: array<Mesh>;
#endif // MESH_BATCH_SIZE

#else // MESH_BINDGROUP_1

#ifdef MESH_BATCH_SIZE
@group(2) @binding(0)
var<uniform> mesh: array<Mesh, #{MESH_BATCH_SIZE}u>;
#else
@group(2) @binding(0)
var<storage> mesh: array<Mesh>;
#endif // MESH_BATCH_SIZE

#endif // MESH_BINDGROUP_1
