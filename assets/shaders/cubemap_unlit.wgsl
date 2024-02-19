#import bevy_pbr::forward_io::VertexOutput

#ifdef CUBEMAP_ARRAY
@group(2) @binding(0) var base_color_texture: texture_cube_array<f32>;
#else
@group(2) @binding(0) var base_color_texture: texture_cube<f32>;
#endif

@group(2) @binding(1) var base_color_sampler: sampler;

@fragment
fn fragment(
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    let fragment_position_view_lh = mesh.world_position.xyz * vec3<f32>(1.0, 1.0, -1.0);
    return textureSample(
        base_color_texture,
        base_color_sampler,
        fragment_position_view_lh
    );
}
