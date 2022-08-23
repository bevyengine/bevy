#define_import_path bevy_pbr::mesh_bindings

#import bevy_pbr::mesh_types

#ifdef MESH_BINDGROUP_1

@group(1) @binding(0)
var<uniform> mesh: bevy_pbr::mesh_types::Mesh;

#else

@group(2) @binding(0)
var<uniform> mesh: bevy_pbr::mesh_types::Mesh;

#endif