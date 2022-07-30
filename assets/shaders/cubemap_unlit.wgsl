#import bevy_pbr::mesh_view_bindings

#ifdef CUBEMAP_ARRAY
@group(1) @binding(0)
var base_color_texture: texture_cube_array<f32>;
#else
@group(1) @binding(0)
var base_color_texture: texture_cube<f32>;
#endif

@group(1) @binding(1)
var base_color_sampler: sampler;

@fragment
fn fragment(
    #import bevy_pbr::mesh_vertex_output
) -> @location(0) vec4<f32> {
    let fragment_position_view_lh = world_position.xyz * vec3<f32>(1.0, 1.0, -1.0);
    return textureSample(
        base_color_texture,
        base_color_sampler,
        fragment_position_view_lh
    );
}
