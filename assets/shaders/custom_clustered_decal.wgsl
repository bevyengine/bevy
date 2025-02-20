// This shader, a part of the `clustered_decals` example, shows how to use the
// decal `tag` field to apply arbitrary decal effects.

#import bevy_pbr::{
    clustered_forward,
    decal::clustered,
    forward_io::{VertexOutput, FragmentOutput},
    mesh_view_bindings,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // Generate a `PbrInput` struct from the `StandardMaterial` bindings.
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // Alpha discard.
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    // Apply the normal decals.
    pbr_input.material.base_color = clustered::apply_decal_base_color(
        in.world_position.xyz,
        in.position.xy,
        pbr_input.material.base_color
    );

    // Here we tint the color based on the tag of the decal.
    // We could optionally do other things, such as adjust the normal based on a normal map.
    let view_z = clustered::get_view_z(in.world_position.xyz);
    let is_orthographic = clustered::view_is_orthographic();
    let cluster_index =
        clustered_forward::fragment_cluster_index(in.position.xy, view_z, is_orthographic);
    var clusterable_object_index_ranges =
        clustered_forward::unpack_clusterable_object_index_ranges(cluster_index);
    var decal_iterator = clustered::clustered_decal_iterator_new(
        in.world_position.xyz,
        &clusterable_object_index_ranges
    );
    while (clustered::clustered_decal_iterator_next(&decal_iterator)) {
        var decal_base_color = textureSampleLevel(
            mesh_view_bindings::clustered_decal_textures[decal_iterator.texture_index],
            mesh_view_bindings::clustered_decal_sampler,
            decal_iterator.uv,
            0.0
        );

        switch (decal_iterator.tag) {
            case 1u: {
                // Tint with red.
                decal_base_color = vec4(
                    mix(pbr_input.material.base_color.rgb, vec3(1.0, 0.0, 0.0), 0.5),
                    decal_base_color.a,
                );
            }
            case 2u: {
                // Tint with blue.
                decal_base_color = vec4(
                    mix(pbr_input.material.base_color.rgb, vec3(0.0, 0.0, 1.0), 0.5),
                    decal_base_color.a,
                );
            }
            default: {}
        }

        pbr_input.material.base_color = vec4(
            mix(pbr_input.material.base_color.rgb, decal_base_color.rgb, decal_base_color.a),
            pbr_input.material.base_color.a + decal_base_color.a
        );
    }

    // Apply lighting.
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);

    // Apply in-shader post processing (fog, alpha-premultiply, and also
    // tonemapping, debanding if the camera is non-HDR). Note this does not
    // include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);

    return out;
}

