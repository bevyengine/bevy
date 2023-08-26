#define_import_path bevy_sprite::mesh2d_bindings

#import bevy_sprite::mesh2d_types

#ifdef MESH_BINDGROUP_1
@group(1) @binding(0)
var<uniform> mesh: bevy_sprite::mesh2d_types::Mesh2d;

#else
@group(2) @binding(0)
var<uniform> mesh: bevy_sprite::mesh2d_types::Mesh2d;

#endif
