#define_import_path bevy_render::core_bindings

#import bevy_render::core_types as types

@group(0) @binding(auto)
var<uniform> view: types::View;

@group(0) @binding(auto)
var<uniform> globals: types::Globals;
