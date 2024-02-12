#import bevy_pbr::pbr_bindings::{base_color_texture, base_color_sampler}
#import bevy_pbr::pbr_fragment::pbr_input_from_standard_material
#import bevy_pbr::pbr_functions::apply_lighting_and_postprocessing
#import bevy_pbr::mesh_view_bindings::view

#ifdef PREPASS_PIPELINE
#import bevy_pbr::prepass_io::{VertexOutput, FragmentOutput}
#else
#import bevy_pbr::forward_io::{VertexOutput, FragmentOutput}
#endif

@group(2) @binding(100) var decal_texture: texture_2d<f32>;
@group(2) @binding(101) var decal_sampler: sampler;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr = pbr_input_from_standard_material(in, is_front);

    // Modify PBR as necessary.
    var color = textureSampleBias(base_color_texture, base_color_sampler, in.uv, view.mip_bias);
    let decal_uv = (in.uv + vec2(-0.1, -0.2)) * 4.0;
    if (all(decal_uv >= vec2(0.0)) && all(decal_uv <= vec2(1.0))) {
        let decal = textureSampleBias(decal_texture, decal_sampler, decal_uv, view.mip_bias);
        color = vec4(mix(color.rgb, decal.rgb, decal.a), color.a);
    }
    pbr.material.base_color = color;

    return apply_lighting_and_postprocessing(pbr);
}
