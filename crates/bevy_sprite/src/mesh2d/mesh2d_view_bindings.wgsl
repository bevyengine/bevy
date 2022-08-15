#define_import_path bevy_sprite::mesh2d_view_bindings

#import bevy_sprite::mesh2d_view_types as ViewTypes

@group(0) @binding(0)
var<uniform> view: ViewTypes::View;
