#define_import_path bevy_pbr::mesh_bindings

#import bevy_pbr::mesh_types Mesh

#ifdef MESH_BINDGROUP_1

#ifdef PER_OBJECT_BUFFER_BATCH_SIZE
@group(1) @binding(0) var<uniform> mesh: array<Mesh, #{PER_OBJECT_BUFFER_BATCH_SIZE}u>;
#else
@group(1) @binding(0) var<storage> mesh: array<Mesh>;
#endif // PER_OBJECT_BUFFER_BATCH_SIZE

#else // MESH_BINDGROUP_1

#ifdef PER_OBJECT_BUFFER_BATCH_SIZE
@group(2) @binding(0) var<uniform> mesh: array<Mesh, #{PER_OBJECT_BUFFER_BATCH_SIZE}u>;
#else
@group(2) @binding(0) var<storage> mesh: array<Mesh>;
#endif // PER_OBJECT_BUFFER_BATCH_SIZE

#endif // MESH_BINDGROUP_1
