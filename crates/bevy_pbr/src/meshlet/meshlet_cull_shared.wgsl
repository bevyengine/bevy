#define_import_path bevy_pbr::meshlet_cull_shared

#import bevy_pbr::meshlet_bindings::{
    MeshletAabb,
    DispatchIndirectArgs,
    InstancedOffset,
    view,
    previous_view,
    meshlet_instance_uniforms,
}
#import bevy_render::maths::affine3_to_square

// https://github.com/zeux/meshoptimizer/blob/1e48e96c7e8059321de492865165e9ef071bffba/demo/nanite.cpp#L115
fn lod_error_is_imperceptible(lod_sphere: vec4<f32>, simplification_error: f32, instance_id: u32) -> bool {
    let world_from_local = affine3_to_square(meshlet_instance_uniforms[instance_id].world_from_local);
    let world_scale = max(length(world_from_local[0]), max(length(world_from_local[1]), length(world_from_local[2])));
    let sphere_world_space = (world_from_local * vec4(lod_sphere.xyz, 1.0)).xyz;
    let radius_world_space = world_scale * lod_sphere.w;
    let error_world_space = world_scale * simplification_error;

    var projected_error = error_world_space;
    if view.clip_from_view[3][3] != 1.0 {
        // Perspective
        let distance_to_closest_point_on_sphere = distance(sphere_world_space, view.world_position) - radius_world_space;
        let distance_to_closest_point_on_sphere_clamped_to_znear = max(distance_to_closest_point_on_sphere, view.clip_from_view[3][2]);
        projected_error /= distance_to_closest_point_on_sphere_clamped_to_znear;
    }
    projected_error *= view.clip_from_view[1][1] * 0.5;
    projected_error *= view.viewport.w;

    return projected_error < 1.0;
}

fn normalize_plane(p: vec4<f32>) -> vec4<f32> {
    return p / length(p.xyz);
}

// https://fgiesen.wordpress.com/2012/08/31/frustum-planes-from-the-projection-matrix/
// https://fgiesen.wordpress.com/2010/10/17/view-frustum-culling/
fn aabb_in_frustum(aabb: MeshletAabb, instance_id: u32) -> bool {
    let world_from_local = affine3_to_square(meshlet_instance_uniforms[instance_id].world_from_local);
    let clip_from_local = view.clip_from_world * world_from_local;
    let row_major = transpose(clip_from_local);
    let planes = array(
        row_major[3] + row_major[0],
        row_major[3] - row_major[0],
        row_major[3] + row_major[1],
        row_major[3] - row_major[1],
        row_major[2],
    );

    for (var i = 0; i < 5; i++) {
        let plane = normalize_plane(planes[i]);
        let flipped = aabb.half_extent * sign(plane.xyz);
        if dot(aabb.center + flipped, plane.xyz) <= -plane.w {
            return false;
        }
    }
    return true;
}

fn occlusion_cull_clip_from_local(instance_id: u32) -> mat4x4<f32> {
#ifdef FIRST_CULLING_PASS
    let prev_world_from_local = affine3_to_square(meshlet_instance_uniforms[instance_id].previous_world_from_local);
    return previous_view.clip_from_world * prev_world_from_local;
#else
    let world_from_local = affine3_to_square(meshlet_instance_uniforms[instance_id].world_from_local);
    return view.clip_from_world * world_from_local;
#endif
}

fn should_occlusion_cull_aabb(aabb: MeshletAabb, instance_id: u32) -> bool {
    let clip_from_local = occlusion_cull_clip_from_local(instance_id);
    return false;
}
