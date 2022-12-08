#define_import_path bevy_pbr::mesh_bindings

#from bevy_pbr::mesh_types import Mesh

#ifdef MESH_BINDGROUP_1

@group(1) @binding(0)
var<uniform> mesh: ::Mesh;

#else

@group(2) @binding(0)
var<uniform> mesh: ::Mesh;

#endif