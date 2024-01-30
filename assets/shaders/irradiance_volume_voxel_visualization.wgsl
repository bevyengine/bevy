#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::irradiance_volume
#import bevy_pbr::mesh_view_bindings

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // `irradiance_volume_light()` multiplies by intensity, so cancel it out.
    // If we take intensity into account, these spheres will be way too bright.
    let rgb = irradiance_volume::irradiance_volume_light(
        mesh.world_position.xyz,
        mesh.world_normal) / mesh_view_bindings::light_probes.irradiance_volumes[0].intensity;

    return vec4<f32>(rgb, 1.0f);
}
