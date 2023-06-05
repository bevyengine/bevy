#define_import_path bevy_pbr::pbr_prepass_functions

// Cutoff used for the premultiplied alpha modes BLEND and ADD.
const PREMULTIPLIED_ALPHA_CUTOFF = 0.05;

// We can use a simplified version of alpha_discard() here since we only need to handle the alpha_cutoff
fn prepass_alpha_discard(in: FragmentInput) {

#ifdef MAY_DISCARD
    var output_color: vec4<f32> = material.base_color;

#ifdef VERTEX_UVS
    if (material.flags & STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u {
        output_color = output_color * textureSample(base_color_texture, base_color_sampler, in.uv);
    }
#endif // VERTEX_UVS

    let alpha_mode = material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode == STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK {
        if output_color.a < material.alpha_cutoff {
            discard;
        }
    } else if (alpha_mode == STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND || alpha_mode == STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ADD) {
        if output_color.a < PREMULTIPLIED_ALPHA_CUTOFF {
            discard;
        }
    } else if alpha_mode == STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED {
        if all(output_color < vec4(PREMULTIPLIED_ALPHA_CUTOFF)) {
            discard;
        }
    }

#endif // MAY_DISCARD
}