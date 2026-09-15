import bevy_render::view::View;
import bevy_render::globals::Globals;

@group(0) @binding(0) public var<uniform> view: View;

@group(0) @binding(1) public var<uniform> globals: Globals;

@group(0) @binding(2) public var dt_lut_texture: texture_3d<f32>;
@group(0) @binding(3) public var dt_lut_sampler: sampler;
