#import bevy_pbr::{forward_io::VertexOutput, mesh_bindings::mesh}
#import bevy_render::bindless::{bindless_samplers_filtering, bindless_textures_2d}

// The custom data that we extract from the ECS and expose to this shader.
struct CustomMaterialData {
    // The tint color for the mesh.
    color: vec3<f32>,
    // Padding to pad this out to a multiple of 16 bytes.
    pad: u32,
}

// The array of material data that Bevy supplies.
//
// We need to declare this type because
// `binding_array<array<CustomMaterialData>>` isn't currently accepted by Naga.
// We have to factor the `array<CustomMaterialData>` out into a separate type.
struct CustomMaterialDataArray {
    data_array: array<CustomMaterialData>,
}

#ifdef BINDLESS

// The bindings table for bindless mode.
//
// These are indexes into the various arrays.
struct CustomMaterialBindings {
    material: u32,              // 0
    // The index of the data for this instance in the `CustomMaterialDataArray`.
    data: u32,                  // 1
    // The index of the color texture for this instance in the
    // `bindless_textures_2d`.
    color_texture: u32,         // 2
    // The index of the sampler for this instance in the
    // `bindless_samplers_filtering`.
    color_texture_sampler: u32, // 3
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<storage> material_indices:
    array<CustomMaterialBindings>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<storage> material_data:
    binding_array<CustomMaterialDataArray>;

#else   // BINDLESS

@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<storage> material_data: CustomMaterialDataArray;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var color_texture_sampler: sampler;

#endif  // BINDLESS

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // The tag stores the index of the data in the data array.
    let tag = mesh[in.instance_index].tag;

#ifdef BINDLESS

    // Look up the bindless slot for this mesh.
    let slot = mesh[in.instance_index].material_and_lightmap_bind_group_slot & 0xffffu;

    // Unpack the bindless indices.
    let data_index = material_indices[slot].data;
    let color_texture_index = material_indices[slot].color_texture;
    let color_texture_sampler_index = material_indices[slot].color_texture_sampler;

    // Grab the color from the material data array.
    //
    // Note that there are two levels of arrays here: the binding array and the
    // data array. The binding array should only have one element in this case,
    // but Bevy still supplies a binding array because it's legal for different
    // materials to have different `ShaderBuffer` bindings, even though in our
    // case all materials are bound to the same buffer. The data array contains
    // all of the instance data and is indexed by the tag.
    let color = material_data[data_index].data_array[tag].color;

    // Sample from the appropriate texture in the bindless textures array.
    let texture_color = textureSample(
        bindless_textures_2d[color_texture_index],
        bindless_samplers_filtering[color_texture_sampler_index],
        in.uv
    ).rgb;

#else   // BINDLESS

    // In non-bindless mode, we only have one buffer, so we simply use the tag
    // as an index into the array.
    let color = material_data[tag].color;

    let texture_color = textureSample(color_texture, color_texture_sampler, in.uv).rgb;

#endif  // BINDLESS

    // Modulate the texture by the appropriate tint color.
    return vec4<f32>(color * texture_color, 1.0);
}
