import bevy_pbr::render::forward_io::VertexOutput;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) public var test_texture_1d: texture_1d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) public var test_texture_1d_sampler: sampler;

@group(constants::MATERIAL_BIND_GROUP) @binding(2) public var test_texture_2d: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) public var test_texture_2d_sampler: sampler;

@group(constants::MATERIAL_BIND_GROUP) @binding(4) public var test_texture_2d_array: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(5) public var test_texture_2d_array_sampler: sampler;

@group(constants::MATERIAL_BIND_GROUP) @binding(6) public var test_texture_cube: texture_cube<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(7) public var test_texture_cube_sampler: sampler;

@group(constants::MATERIAL_BIND_GROUP) @binding(8) public var test_texture_cube_array: texture_cube_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(9) public var test_texture_cube_array_sampler: sampler;

@group(constants::MATERIAL_BIND_GROUP) @binding(10) public var test_texture_3d: texture_3d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(11) public var test_texture_3d_sampler: sampler;

@fragment
public fn fragment(in: VertexOutput) {}
