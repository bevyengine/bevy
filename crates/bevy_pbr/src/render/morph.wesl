@if(MORPH_TARGETS) import bevy_pbr::render::mesh_types::{MorphAttributes, MorphDescriptor, MorphWeights};

@if(MORPH_TARGETS && SKINS_USE_UNIFORM_BUFFERS)
@group(2) @binding(2) var<uniform> morph_weights: MorphWeights;
@if(MORPH_TARGETS && SKINS_USE_UNIFORM_BUFFERS)
@group(2) @binding(3) var morph_targets: texture_3d<f32>;
@if(MORPH_TARGETS && SKINS_USE_UNIFORM_BUFFERS)
@group(2) @binding(7) var<uniform> prev_morph_weights: MorphWeights;

@if(MORPH_TARGETS && !SKINS_USE_UNIFORM_BUFFERS)
@group(2) @binding(2) var<storage> morph_weights: array<f32>;
@if(MORPH_TARGETS && !SKINS_USE_UNIFORM_BUFFERS)
@group(2) @binding(3) var<storage> morph_targets: array<MorphAttributes>;
@if(MORPH_TARGETS && !SKINS_USE_UNIFORM_BUFFERS)
@group(2) @binding(7) var<storage> prev_morph_weights: array<f32>;
@if(MORPH_TARGETS && !SKINS_USE_UNIFORM_BUFFERS)
@group(2) @binding(8) var<storage> morph_descriptors: array<MorphDescriptor>;

// NOTE: Those are the "hardcoded" values found in `MorphAttributes` struct
// in crates/bevy_render/src/mesh/morph/visitors.rs
// In an ideal world, the offsets are established dynamically and passed as #defines
// to the shader, but it's out of scope for the initial implementation of morph targets.
@if(MORPH_TARGETS)
const position_offset: u32 = 0u;
@if(MORPH_TARGETS)
const normal_offset: u32 = 3u;
@if(MORPH_TARGETS)
const tangent_offset: u32 = 6u;
@if(MORPH_TARGETS)
const total_component_count: u32 = 9u;

@if(MORPH_TARGETS)
fn layer_count(descriptor_index: u32) -> u32 {
@if(SKINS_USE_UNIFORM_BUFFERS)
    let dimensions = textureDimensions(morph_targets);
@if(SKINS_USE_UNIFORM_BUFFERS)
    return u32(dimensions.z);
@if(!SKINS_USE_UNIFORM_BUFFERS)
    return morph_descriptors[descriptor_index].weight_count;
}

@if(MORPH_TARGETS && SKINS_USE_UNIFORM_BUFFERS)
fn component_texture_coord(vertex_index: u32, component_offset: u32) -> vec2<u32> {
    let width = u32(textureDimensions(morph_targets).x);
    let component_index = total_component_count * vertex_index + component_offset;
    return vec2<u32>(component_index % width, component_index / width);
}

@if(MORPH_TARGETS)
fn weight_at(weight_index: u32, descriptor_index: u32) -> f32 {
@if(SKINS_USE_UNIFORM_BUFFERS)
    let i = weight_index;
@if(SKINS_USE_UNIFORM_BUFFERS)
    return morph_weights.weights[i / 4u][i % 4u];
@if(!SKINS_USE_UNIFORM_BUFFERS)
    let weights_offset = morph_descriptors[descriptor_index].current_weights_offset;
@if(!SKINS_USE_UNIFORM_BUFFERS)
    return morph_weights[weights_offset + weight_index];
}

@if(MORPH_TARGETS)
fn prev_weight_at(weight_index: u32, descriptor_index: u32) -> f32 {
@if(SKINS_USE_UNIFORM_BUFFERS)
    let i = weight_index;
@if(SKINS_USE_UNIFORM_BUFFERS)
    return prev_morph_weights.weights[i / 4u][i % 4u];
@if(!SKINS_USE_UNIFORM_BUFFERS)
    let weights_offset = morph_descriptors[descriptor_index].prev_weights_offset;
@if(!SKINS_USE_UNIFORM_BUFFERS)
    return prev_morph_weights[weights_offset + weight_index];
}

@if(MORPH_TARGETS && SKINS_USE_UNIFORM_BUFFERS)
fn morph_pixel(vertex: u32, component: u32, weight: u32) -> f32 {
    let coord = component_texture_coord(vertex, component);
    // Due to https://gpuweb.github.io/gpuweb/wgsl/#texel-formats
    // While the texture stores a f32, the textureLoad returns a vec4<>, where
    // only the first component is set.
    return textureLoad(morph_targets, vec3(coord, weight), 0).r;
}

@if(MORPH_TARGETS && SKINS_USE_UNIFORM_BUFFERS)
fn morph(vertex_index: u32, component_offset: u32, weight_index: u32) -> vec3<f32> {
    return vec3<f32>(
        morph_pixel(vertex_index, component_offset, weight_index),
        morph_pixel(vertex_index, component_offset + 1u, weight_index),
        morph_pixel(vertex_index, component_offset + 2u, weight_index),
    );
}

@if(MORPH_TARGETS && SKINS_USE_UNIFORM_BUFFERS)
fn morph_position(vertex_index: u32, weight_index: u32, instance_index: u32) -> vec3<f32> {
    return morph(vertex_index, position_offset, weight_index);
}

@if(MORPH_TARGETS && SKINS_USE_UNIFORM_BUFFERS)
fn morph_normal(vertex_index: u32, weight_index: u32, instance_index: u32) -> vec3<f32> {
    return morph(vertex_index, normal_offset, weight_index);
}

@if(MORPH_TARGETS && SKINS_USE_UNIFORM_BUFFERS)
fn morph_tangent(vertex_index: u32, weight_index: u32, instance_index: u32) -> vec3<f32> {
    return morph(vertex_index, tangent_offset, weight_index);
}

@if(MORPH_TARGETS && !SKINS_USE_UNIFORM_BUFFERS)
fn get_morph_target(vertex_index: u32, weight_index: u32, descriptor_index: u32) -> MorphAttributes {
    let targets_offset = morph_descriptors[descriptor_index].targets_offset;
    let vertex_count = morph_descriptors[descriptor_index].vertex_count;
    return morph_targets[targets_offset + weight_index * vertex_count + vertex_index];
}

@if(MORPH_TARGETS && !SKINS_USE_UNIFORM_BUFFERS)
fn morph_position(vertex_index: u32, weight_index: u32, descriptor_index: u32) -> vec3<f32> {
    return get_morph_target(vertex_index, weight_index, descriptor_index).position;
}

@if(MORPH_TARGETS && !SKINS_USE_UNIFORM_BUFFERS)
fn morph_normal(vertex_index: u32, weight_index: u32, descriptor_index: u32) -> vec3<f32> {
    return get_morph_target(vertex_index, weight_index, descriptor_index).normal;
}

@if(MORPH_TARGETS && !SKINS_USE_UNIFORM_BUFFERS)
fn morph_tangent(vertex_index: u32, weight_index: u32, descriptor_index: u32) -> vec3<f32> {
    return get_morph_target(vertex_index, weight_index, descriptor_index).tangent;
}
