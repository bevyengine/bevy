#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_bindings

#import bevy_pbr::pbr_types
#import bevy_pbr::utils
#import bevy_pbr::clustered_forward
#import bevy_pbr::lighting
#import bevy_pbr::shadows
#import bevy_pbr::fog
#import bevy_pbr::pbr_functions
#import bevy_pbr::pbr_ambient

@group(1) @binding(0)
var my_array_texture: texture_2d_array<f32>;
@group(1) @binding(1)
var my_array_texture_sampler: sampler;

struct FragmentInput {
    @builtin(front_facing) is_front: bool,
    @builtin(position) frag_coord: vec4<f32>,
    #import bevy_pbr::mesh_vertex_output
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    let layer = i32(in.world_position.x) & 0x3;

    // Prepare a 'processed' StandardMaterial by sampling all textures to resolve
    // the material members
    var pbr_input: PbrInput = pbr_input_new();

    pbr_input.material.base_color = textureSample(my_array_texture, my_array_texture_sampler, in.uv, layer);
#ifdef VERTEX_COLORS
    pbr_input.material.base_color = pbr_input.material.base_color * in.color;
#endif

    pbr_input.frag_coord = in.frag_coord;
    pbr_input.world_position = in.world_position;
    pbr_input.world_normal = prepare_world_normal(
        in.world_normal,
        in.is_front,
        pbr_input.material.flags
    );

    pbr_input.is_orthographic = view.projection[3].w == 1.0;

    var N = normalize(pbr_input.world_normal);
#ifdef VERTEX_TANGENTS
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_NORMAL_MAP_TEXTURE_BIT) != 0u {
        let tangent_normal = textureSample(normal_map_texture, normal_map_sampler, in.uv).rgb;
        N = apply_normal_mapping(
            pbr_input.material.flags,
            pbr_input.world_normal,
            in.world_tangent,
            tangent_normal,
        );
    }
#endif // VERTEX_TANGENTS
    pbr_input.N = N;
    pbr_input.V = calculate_view(in.world_position, pbr_input.is_orthographic);

    return tone_mapping(pbr(pbr_input));
}
