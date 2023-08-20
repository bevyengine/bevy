#define_import_path bevy_sprite::mesh2d_bindings

#import bevy_sprite::mesh2d_types

@group(2) @binding(0)
var<uniform> mesh: bevy_sprite::mesh2d_types::Mesh2d;
