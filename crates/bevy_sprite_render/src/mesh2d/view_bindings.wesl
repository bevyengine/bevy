import bevy_render::view::View;
import bevy_render::globals::Globals;

@if(LIGHTING_2D)
import package::light::types::Lights2d;

@group(0) @binding(0) var<uniform> view: View;

@group(0) @binding(1) var<uniform> globals: Globals;

@group(0) @binding(2) var dt_lut_texture: texture_3d<f32>;
@group(0) @binding(3) var dt_lut_sampler: sampler;

@if(LIGHTING_2D)
@group(0) @binding(4) var<uniform> lights_2d: Lights2d;
