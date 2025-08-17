#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::irradiance_volume
#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::clustered_forward

struct VoxelVisualizationIrradianceVolumeInfo {
    world_from_voxel: mat4x4<f32>,
    voxel_from_world: mat4x4<f32>,
    resolution: vec3<u32>,
    // A scale factor that's applied to the diffuse and specular light from the
    // light probe. This is in units of cd/m² (candela per square meter).
    intensity: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> irradiance_volume_info: VoxelVisualizationIrradianceVolumeInfo;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // Snap the world position we provide to `irradiance_volume_light()` to the
    // middle of the nearest texel.
    var unit_pos = (irradiance_volume_info.voxel_from_world *
        vec4(mesh.world_position.xyz, 1.0f)).xyz;
    let resolution = vec3<f32>(irradiance_volume_info.resolution);
    let stp = clamp((unit_pos + 0.5) * resolution, vec3(0.5f), resolution - vec3(0.5f));
    let stp_rounded = round(stp - 0.5f) + 0.5f;
    let rounded_world_pos = (irradiance_volume_info.world_from_voxel * vec4(stp_rounded, 1.0f)).xyz;

    // Look up the irradiance volume range in the cluster list.
    let view_z = dot(vec4<f32>(
        mesh_view_bindings::view.view_from_world[0].z,
        mesh_view_bindings::view.view_from_world[1].z,
        mesh_view_bindings::view.view_from_world[2].z,
        mesh_view_bindings::view.view_from_world[3].z
    ), mesh.world_position);
    let cluster_index = clustered_forward::fragment_cluster_index(mesh.position.xy, view_z, false);
    var clusterable_object_index_ranges =
        clustered_forward::unpack_clusterable_object_index_ranges(cluster_index);

    // `irradiance_volume_light()` multiplies by intensity, so cancel it out.
    // If we take intensity into account, the cubes will be way too bright.
    let rgb = irradiance_volume::irradiance_volume_light(
        mesh.world_position.xyz,
        mesh.world_normal,
        &clusterable_object_index_ranges,
    ) / irradiance_volume_info.intensity;

    return vec4<f32>(rgb, 1.0f);
}
