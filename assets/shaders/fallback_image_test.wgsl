#import bevy_pbr::forward_io::VertexOutput

@group(2) @binding(0) var test_texture_1d: texture_1d<f32>;
@group(2) @binding(1) var test_texture_1d_sampler: sampler;

@group(2) @binding(2) var test_texture_2d: texture_2d<f32>;
@group(2) @binding(3) var test_texture_2d_sampler: sampler;

@group(2) @binding(4) var test_texture_2d_array: texture_2d_array<f32>;
@group(2) @binding(5) var test_texture_2d_array_sampler: sampler;

@group(2) @binding(6) var test_texture_cube: texture_cube<f32>;
@group(2) @binding(7) var test_texture_cube_sampler: sampler;

@group(2) @binding(8) var test_texture_cube_array: texture_cube_array<f32>;
@group(2) @binding(9) var test_texture_cube_array_sampler: sampler;

@group(2) @binding(10) var test_texture_3d: texture_3d<f32>;
@group(2) @binding(11) var test_texture_3d_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) {}
