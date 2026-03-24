#define_import_path bevy_pbr::morph

#ifdef MORPH_TARGETS

#import bevy_pbr::mesh_types::{MorphAttributes, MorphDescriptor, MorphWeights}
#import bevy_pbr::mesh_bindings::mesh

#ifdef SKINS_USE_UNIFORM_BUFFERS

@group(2) @binding(2) var<uniform> morph_weights: MorphWeights;
@group(2) @binding(3) var morph_targets: texture_3d<f32>;
@group(2) @binding(7) var<uniform> prev_morph_weights: MorphWeights;

#else   // SKINS_USE_UNIFORM_BUFFERS

@group(2) @binding(2) var<storage> morph_weights: array<f32>;
@group(2) @binding(3) var<storage> morph_targets: array<MorphAttributes>;
@group(2) @binding(7) var<storage> prev_morph_weights: array<f32>;
@group(2) @binding(8) var<storage> morph_descriptors: array<MorphDescriptor>;

#endif  // SKINS_USE_UNIFORM_BUFFERS

// NOTE: Those are the "hardcoded" values found in `MorphAttributes` struct
// in crates/bevy_render/src/mesh/morph/visitors.rs
// In an ideal world, the offsets are established dynamically and passed as #defines
// to the shader, but it's out of scope for the initial implementation of morph targets.
const position_offset: u32 = 0u;
const normal_offset: u32 = 3u;
const tangent_offset: u32 = 6u;
const total_component_count: u32 = 9u;

fn layer_count(instance_index: u32) -> u32 {
#ifdef SKINS_USE_UNIFORM_BUFFERS
    let dimensions = textureDimensions(morph_targets);
    return u32(dimensions.z);
#else   // SKINS_USE_UNIFORM_BUFFERS
    let morph_descriptor_index = mesh[instance_index].morph_descriptor_index;
    return morph_descriptors[morph_descriptor_index].weight_count;
#endif  // SKINS_USE_UNIFORM_BUFFERS
}

#ifdef SKINS_USE_UNIFORM_BUFFERS
fn component_texture_coord(vertex_index: u32, component_offset: u32) -> vec2<u32> {
    let width = u32(textureDimensions(morph_targets).x);
    let component_index = total_component_count * vertex_index + component_offset;
    return vec2<u32>(component_index % width, component_index / width);
}
#endif  // SKINS_USE_UNIFORM_BUFFERS

fn weight_at(weight_index: u32, instance_index: u32) -> f32 {
#ifdef SKINS_USE_UNIFORM_BUFFERS
    let i = weight_index;
    return morph_weights.weights[i / 4u][i % 4u];
#else   // SKINS_USE_UNIFORM_BUFFERS
    let morph_descriptor_index = mesh[instance_index].morph_descriptor_index;
    let weights_offset = morph_descriptors[morph_descriptor_index].current_weights_offset;
    return morph_weights[weights_offset + weight_index];
#endif  // SKINS_USE_UNIFORM_BUFFERS
}

fn prev_weight_at(weight_index: u32, instance_index: u32) -> f32 {
#ifdef SKINS_USE_UNIFORM_BUFFERS
    let i = weight_index;
    return prev_morph_weights.weights[i / 4u][i % 4u];
#else   // SKINS_USE_UNIFORM_BUFFERS
    let morph_descriptor_index = mesh[instance_index].morph_descriptor_index;
    let weights_offset = morph_descriptors[morph_descriptor_index].prev_weights_offset;
    return prev_morph_weights[weights_offset + weight_index];
#endif  // SKINS_USE_UNIFORM_BUFFERS
}

#ifdef SKINS_USE_UNIFORM_BUFFERS

fn morph_pixel(vertex: u32, component: u32, weight: u32) -> f32 {
    let coord = component_texture_coord(vertex, component);
    // Due to https://gpuweb.github.io/gpuweb/wgsl/#texel-formats
    // While the texture stores a f32, the textureLoad returns a vec4<>, where
    // only the first component is set.
    return textureLoad(morph_targets, vec3(coord, weight), 0).r;
}

fn morph(vertex_index: u32, component_offset: u32, weight_index: u32) -> vec3<f32> {
    return vec3<f32>(
        morph_pixel(vertex_index, component_offset, weight_index),
        morph_pixel(vertex_index, component_offset + 1u, weight_index),
        morph_pixel(vertex_index, component_offset + 2u, weight_index),
    );
}

fn morph_position(vertex_index: u32, weight_index: u32, instance_index: u32) -> vec3<f32> {
    return morph(vertex_index, position_offset, weight_index);
}

fn morph_normal(vertex_index: u32, weight_index: u32, instance_index: u32) -> vec3<f32> {
    return morph(vertex_index, normal_offset, weight_index);
}

fn morph_tangent(vertex_index: u32, weight_index: u32, instance_index: u32) -> vec3<f32> {
    return morph(vertex_index, tangent_offset, weight_index);
}

#else   // SKINS_USE_UNIFORM_BUFFERS

fn get_morph_target(vertex_index: u32, weight_index: u32, instance_index: u32) -> MorphAttributes {
    let morph_descriptor_index = mesh[instance_index].morph_descriptor_index;
    let targets_offset = morph_descriptors[morph_descriptor_index].targets_offset;
    let vertex_count = morph_descriptors[morph_descriptor_index].vertex_count;
    return morph_targets[targets_offset + weight_index * vertex_count + vertex_index];
}

fn morph_position(vertex_index: u32, weight_index: u32, instance_index: u32) -> vec3<f32> {
    return get_morph_target(vertex_index, weight_index, instance_index).position;
}

fn morph_normal(vertex_index: u32, weight_index: u32, instance_index: u32) -> vec3<f32> {
    return get_morph_target(vertex_index, weight_index, instance_index).normal;
}

fn morph_tangent(vertex_index: u32, weight_index: u32, instance_index: u32) -> vec3<f32> {
    return get_morph_target(vertex_index, weight_index, instance_index).tangent;
}

#endif  // SKINS_USE_UNIFORM_BUFFERS

#endif // MORPH_TARGETS
