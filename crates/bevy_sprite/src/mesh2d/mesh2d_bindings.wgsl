#define_import_path bevy_sprite::mesh2d_bindings

#import bevy_sprite::mesh2d_types as MeshTypes

@group(2) @binding(0)
var<uniform> mesh: MeshTypes::Mesh2d;
