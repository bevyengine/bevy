// The shader that goes with `extended_material_bindless.rs`.

#import bevy_pbr::{
    forward_io::{FragmentOutput, VertexOutput},
    mesh_bindings::mesh,
    pbr_bindings::{material_array, material_indices},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#import bevy_render::bindless::{bindless_samplers_filtering, bindless_textures_2d}

// Stores the indices of the bindless resources in the bindless resource arrays,
// for the `ExampleBindlessExtension` fields.
struct ExampleBindlessExtendedMaterialIndices {
    // The index of the `ExampleBindlessExtendedMaterial` data in
    // `example_extended_material`.
    material: u32,
    // The index of the texture we're going to modulate the base color with in
    // the `bindless_textures_2d` array.
    modulate_texture: u32,
    // The index of the sampler we're going to sample the modulated texture with
    // in the `bindless_samplers_filtering` array.
    modulate_texture_sampler: u32,
}

// Plain data associated with this example material.
struct ExampleBindlessExtendedMaterial {
    // The color that we multiply the base color, base color texture, and
    // modulated texture with.
    modulate_color: vec4<f32>,
}

// The indices of the bindless resources in the bindless resource arrays, for
// the `ExampleBindlessExtension` fields.
@group(2) @binding(100) var<storage> example_extended_material_indices:
    array<ExampleBindlessExtendedMaterialIndices>;
// An array that holds the `ExampleBindlessExtendedMaterial` plain old data,
// indexed by `ExampleBindlessExtendedMaterialIndices.material`.
@group(2) @binding(101) var<storage> example_extended_material:
    array<ExampleBindlessExtendedMaterial>;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // Fetch the material slot. We'll use this in turn to fetch the bindless
    // indices from `example_extended_material_indices`.
    let slot = mesh[in.instance_index].material_and_lightmap_bind_group_slot & 0xffffu;

    // Generate a `PbrInput` struct from the `StandardMaterial` bindings.
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // Calculate the UV for the texture we're about to sample.
    let uv_transform = material_array[material_indices[slot].material].uv_transform;
    let uv = (uv_transform * vec3(in.uv, 1.0)).xy;

    // Multiply the base color by the `modulate_texture` and `modulate_color`.
    // Notice how we fetch the texture, sampler, and plain extended material
    // data from the appropriate arrays.
    pbr_input.material.base_color *= textureSample(
        bindless_textures_2d[example_extended_material_indices[slot].modulate_texture],
        bindless_samplers_filtering[
            example_extended_material_indices[slot].modulate_texture_sampler
        ],
        uv
    ) * example_extended_material[example_extended_material_indices[slot].material].modulate_color;

    var out: FragmentOutput;
    // Apply lighting.
    out.color = apply_pbr_lighting(pbr_input);
    // Apply in-shader post processing (fog, alpha-premultiply, and also
    // tonemapping, debanding if the camera is non-HDR). Note this does not
    // include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
