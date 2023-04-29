#define_import_path bevy_sprite::mesh2d_view_bindings

#import bevy_render::view  View
#import bevy_render::globals  Globals

@group(0) @binding(0)
var<uniform> view: View;

@group(0) @binding(1)
var<uniform> globals: Globals;
