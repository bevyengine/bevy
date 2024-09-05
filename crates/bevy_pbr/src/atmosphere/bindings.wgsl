#define_import_path bevy_pbr::atmosphere::bindings

#import bevy_pbr::mesh_view_types::Lights
#import bevy_render::{view::View, globals::Globals}


//TODO: set max directional lights in Lights array, and max cascades per directional light
@group(0) @binding(0) var<uniform> atmosphere: Atmosphere;
