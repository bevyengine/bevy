import bevy_render::bindless::{bindless_samplers_filtering, bindless_textures_2d};

import bevy_pbr::{
    prepass::io::VertexOutput,
    prepass::bindings::previous_view_uniforms,
    render::{
        mesh_bindings::mesh,
        mesh_view_bindings::view,
        pbr_bindings,
        pbr_types,
    },
};

@if(BINDLESS)
import bevy_pbr::render::pbr_bindings::material_indices;

// Cutoff used for the premultiplied alpha modes BLEND, ADD, and ALPHA_TO_COVERAGE.
const PREMULTIPLIED_ALPHA_CUTOFF = 0.05;

// Reusable alpha discard logic suitable for use by custom materials in the prepass.
// We can use a simplified version of alpha_discard() here since we only need to handle the alpha_cutoff
fn prepass_alpha_discard(material_flags: u32, alpha_cutoff: f32, output_color: vec4<f32>) {
    let alpha_mode = material_flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK {
        if output_color.a < alpha_cutoff {
            discard;
        }
    } else if (alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND ||
            alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ADD ||
            alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ALPHA_TO_COVERAGE) {
        if output_color.a < PREMULTIPLIED_ALPHA_CUTOFF {
            discard;
        }
    } else if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED {
        if all(output_color < vec4(PREMULTIPLIED_ALPHA_CUTOFF)) {
            discard;
        }
    }
}

// Samples the StandardMaterial's base color (including the base color texture, if any) and then
// invokes `prepass_alpha_discard` with the material's flags and alpha cutoff.
fn prepass_sample_color_and_alpha_discard(in: VertexOutput) {

@if(MAY_DISCARD) {
@if(BINDLESS)
    let slot = mesh[in.instance_index].material_and_lightmap_bind_group_slot & 0xffffu;
@if(BINDLESS)
    var output_color: vec4<f32> = pbr_bindings::material_array[material_indices[slot].material].base_color;
@if(BINDLESS)
    let flags = pbr_bindings::material_array[material_indices[slot].material].flags;
@if(BINDLESS)
    let alpha_cutoff = pbr_bindings::material_array[material_indices[slot].material].alpha_cutoff;
@if(!BINDLESS)
    var output_color: vec4<f32> = pbr_bindings::material.base_color;
@if(!BINDLESS)
    let flags = pbr_bindings::material.flags;
@if(!BINDLESS)
    let alpha_cutoff = pbr_bindings::material.alpha_cutoff;

@if(VERTEX_UVS) {
@if(STANDARD_MATERIAL_BASE_COLOR_UV_B)
    var uv = in.uv_b;
@else   // STANDARD_MATERIAL_BASE_COLOR_UV_B
    var uv = in.uv;

@if(BINDLESS)
    let uv_transform = pbr_bindings::material_array[material_indices[slot].material].uv_transform;
@else   // BINDLESS
    let uv_transform = pbr_bindings::material.uv_transform;

    uv = (uv_transform * vec3(uv, 1.0)).xy;
    if (flags & pbr_types::STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u {
@if(BINDLESS)
        let texture = bindless_textures_2d[material_indices[slot].base_color_texture];
@else
        let texture = pbr_bindings::base_color_texture;
@if(BINDLESS)
        let sampler_ = bindless_samplers_filtering[material_indices[slot].base_color_sampler];
@else
        let sampler_ = pbr_bindings::base_color_sampler;
        output_color = output_color * textureSampleBias(
            texture,
            sampler_,
            uv,
            view.mip_bias
        );
    }
}

    prepass_alpha_discard(flags, alpha_cutoff, output_color);
}
}

@if(MOTION_VECTOR_PREPASS)
fn calculate_motion_vector(world_position: vec4<f32>, previous_world_position: vec4<f32>) -> vec2<f32> {
    let clip_position_t = view.unjittered_clip_from_world * world_position;
    let clip_position = clip_position_t.xy / clip_position_t.w;
    let previous_clip_position_t = previous_view_uniforms.unjittered_clip_from_world * previous_world_position;
    let previous_clip_position = previous_clip_position_t.xy / previous_clip_position_t.w;
    // These motion vectors are used as offsets to UV positions and are stored
    // in the range -1,1 to allow offsetting from the one corner to the
    // diagonally-opposite corner in UV coordinates, in either direction.
    // A difference between diagonally-opposite corners of clip space is in the
    // range -2,2, so this needs to be scaled by 0.5. And the V direction goes
    // down where clip space y goes up, so y needs to be flipped.
    return (clip_position - previous_clip_position) * vec2(0.5, -0.5);
}
