#import bevy_pbr::mesh_vertex_output
#import bevy_pbr::pbr_functions
#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::pbr_types

@group(1) @binding(0)
var my_array_texture: texture_2d_array<f32>;
@group(1) @binding(1)
var my_array_texture_sampler: sampler;

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: bevy_pbr::mesh_vertex_output::MeshVertexOutput,
) -> @location(0) vec4<f32> {
    let layer = i32(mesh.world_position.x) & 0x3;

    // Prepare a 'processed' StandardMaterial by sampling all textures to resolve
    // the material members
    var pbr_input: bevy_pbr::pbr_functions::PbrInput = bevy_pbr::pbr_functions::pbr_input_new();

    pbr_input.material.base_color = textureSample(my_array_texture, my_array_texture_sampler, mesh.uv, layer);
#ifdef VERTEX_COLORS
    pbr_input.material.base_color = pbr_input.material.base_color * mesh.color;
#endif

    pbr_input.frag_coord = mesh.clip_position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.world_normal = bevy_pbr::pbr_functions::prepare_world_normal(
        mesh.world_normal,
        (pbr_input.material.flags & bevy_pbr::pbr_types::STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u,
        is_front,
    );

    pbr_input.is_orthographic = bevy_pbr::mesh_view_bindings::view.projection[3].w == 1.0;

    pbr_input.N = bevy_pbr::pbr_functions::apply_normal_mapping(
        pbr_input.material.flags,
        mesh.world_normal,
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
        mesh.world_tangent,
#endif
#endif
        mesh.uv,
    );
    pbr_input.V = bevy_pbr::pbr_functions::calculate_view(mesh.world_position, pbr_input.is_orthographic);

    return bevy_pbr::pbr_functions::tone_mapping(bevy_pbr::pbr_functions::pbr(pbr_input));
}
