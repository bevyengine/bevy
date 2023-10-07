// If using this WGSL snippet as an #import, the following should be in scope:
//
// - the `morph_weights` uniform of type `MorphWeights`
// - the `morph_targets` 3d texture
//
// They are defined in `mesh_types.wgsl` and `mesh_bindings.wgsl`.

#define_import_path bevy_pbr::morph

#ifdef MORPH_TARGETS

#import bevy_pbr::mesh_types MorphWeights

#ifdef MESH_BINDGROUP_1

@group(1) @binding(2) var<uniform> morph_weights: MorphWeights;
@group(1) @binding(3) var morph_targets: texture_3d<f32>;

#else

@group(2) @binding(2) var<uniform> morph_weights: MorphWeights;
@group(2) @binding(3) var morph_targets: texture_3d<f32>;

#endif


// NOTE: Those are the "hardcoded" values found in `MorphAttributes` struct
// in crates/bevy_render/src/mesh/morph/visitors.rs
// In an ideal world, the offsets are established dynamically and passed as #defines
// to the shader, but it's out of scope for the initial implementation of morph targets.
const position_offset: u32 = 0u;
const normal_offset: u32 = 3u;
const tangent_offset: u32 = 6u;
const total_component_count: u32 = 9u;

fn layer_count() -> u32 {
    let dimensions = textureDimensions(morph_targets);
    return u32(dimensions.z);
}
fn component_texture_coord(vertex_index: u32, component_offset: u32) -> vec2<u32> {
    let width = u32(textureDimensions(morph_targets).x);
    let component_index = total_component_count * vertex_index + component_offset;
    return vec2<u32>(component_index % width, component_index / width);
}
fn weight_at(weight_index: u32) -> f32 {
    let i = weight_index;
    return morph_weights.weights[i / 4u][i % 4u];
}
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

#endif // MORPH_TARGETS