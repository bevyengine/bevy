#import bevy_pbr::mesh_vertex_output as OutputTypes
#import bevy_pbr::pbr_functions as PbrCore
#import bevy_pbr::mesh_view_bindings as ViewBindings

@group(1) @binding(0)
var my_array_texture: texture_2d_array<f32>;
@group(1) @binding(1)
var my_array_texture_sampler: sampler;

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    @builtin(position) frag_coord: vec4<f32>,
    mesh: OutputTypes::MeshVertexOutput,
) -> @location(0) vec4<f32> {
    let layer = i32(mesh.world_position.x) & 0x3;

    // Prepare a 'processed' StandardMaterial by sampling all textures to resolve
    // the material members
    var pbr_input: PbrCore::PbrInput = PbrCore::pbr_input_new();

    pbr_input.material.base_color = textureSample(my_array_texture, my_array_texture_sampler, mesh.uv, layer);
#ifdef VERTEX_COLORS
    pbr_input.material.base_color = pbr_input.material.base_color * mesh.color;
#endif

    pbr_input.frag_coord = frag_coord;
    pbr_input.world_position = mesh.world_position;
    pbr_input.world_normal = mesh.world_normal;

    pbr_input.is_orthographic = ViewBindings::view.projection[3].w == 1.0;

    pbr_input.N = PbrCore::prepare_normal(
        pbr_input.material.flags,
        mesh.world_normal,
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
        mesh.world_tangent,
#endif
#endif
        mesh.uv,
        is_front,
    );
    pbr_input.V = PbrCore::calculate_view(mesh.world_position, pbr_input.is_orthographic);

    return PbrCore::tone_mapping(PbrCore::pbr(pbr_input));
}
