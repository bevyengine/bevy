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
    return simplification_error == 0.0;
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
