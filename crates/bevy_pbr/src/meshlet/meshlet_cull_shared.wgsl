#define_import_path bevy_pbr::meshlet_cull_shared

#import bevy_pbr::meshlet_bindings::{
    MeshletAabb,
    DispatchIndirectArgs,
    InstancedOffset,
    depth_pyramid,
    view,
    previous_view,
    meshlet_instance_uniforms,
}
#import bevy_render::maths::affine3_to_square

// https://github.com/zeux/meshoptimizer/blob/1e48e96c7e8059321de492865165e9ef071bffba/demo/nanite.cpp#L115
fn lod_error_is_imperceptible(lod_sphere: vec4<f32>, simplification_error: f32, instance_id: u32) -> bool {
    let world_from_local = affine3_to_square(meshlet_instance_uniforms[instance_id].world_from_local);
    let world_scale = max(length(world_from_local[0]), max(length(world_from_local[1]), length(world_from_local[2])));
    let camera_pos = view.world_position;

    let projection = view.clip_from_view;
    if projection[3][3] == 1.0 {
        // Orthographic
        let world_error = simplification_error * world_scale;
        let proj = projection[1][1];
        let height = 2.0 / proj;
        let norm_error = world_error / height;
        return norm_error * view.viewport.w < 1.0;
    } else {
        // Perspective
        var near = projection[3][2];
        let world_sphere_center = (world_from_local * vec4<f32>(lod_sphere.xyz, 1.0)).xyz;
        let world_sphere_radius = lod_sphere.w * world_scale;
        let d_pos = world_sphere_center - camera_pos;
        let d = sqrt(dot(d_pos, d_pos)) - world_sphere_radius;
        let norm_error = simplification_error / max(d, near) * projection[1][1] * 0.5;
        return norm_error * view.viewport.w < 1.0;
    }
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

struct ScreenAabb {
    min: vec3<f32>,
    max: vec3<f32>,
}

fn min8(a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, d: vec3<f32>, e: vec3<f32>, f: vec3<f32>, g: vec3<f32>, h: vec3<f32>) -> vec3<f32> {
    return min(min(min(a, b), min(c, d)), min(min(e, f), min(g, h)));
}

fn max8(a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, d: vec3<f32>, e: vec3<f32>, f: vec3<f32>, g: vec3<f32>, h: vec3<f32>) -> vec3<f32> {
    return max(max(max(a, b), max(c, d)), max(max(e, f), max(g, h)));
}

fn min8_4(a: vec4<f32>, b: vec4<f32>, c: vec4<f32>, d: vec4<f32>, e: vec4<f32>, f: vec4<f32>, g: vec4<f32>, h: vec4<f32>) -> vec4<f32> {
    return min(min(min(a, b), min(c, d)), min(min(e, f), min(g, h)));
}

// https://zeux.io/2023/01/12/approximate-projected-bounds/
fn project_aabb(clip_from_local: mat4x4<f32>, near: f32, aabb: MeshletAabb, out: ptr<function, ScreenAabb>) -> bool {
    let extent = aabb.half_extent * 2.0;
    let sx = clip_from_local * vec4<f32>(extent.x, 0.0, 0.0, 0.0);
    let sy = clip_from_local * vec4<f32>(0.0, extent.y, 0.0, 0.0);
    let sz = clip_from_local * vec4<f32>(0.0, 0.0, extent.z, 0.0);

    let p0 = clip_from_local * vec4<f32>(aabb.center - aabb.half_extent, 1.0);
    let p1 = p0 + sz;
    let p2 = p0 + sy;
    let p3 = p2 + sz;
    let p4 = p0 + sx;
    let p5 = p4 + sz;
    let p6 = p4 + sy;
    let p7 = p6 + sz;

    let depth = min8_4(p0, p1, p2, p3, p4, p5, p6, p7).w;
    // do not occlusion cull if we are inside the aabb
    if depth < near {
        return false;
    }

    let dp0 = p0.xyz / p0.w;
    let dp1 = p1.xyz / p1.w;
    let dp2 = p2.xyz / p2.w;
    let dp3 = p3.xyz / p3.w;
    let dp4 = p4.xyz / p4.w;
    let dp5 = p5.xyz / p5.w;
    let dp6 = p6.xyz / p6.w;
    let dp7 = p7.xyz / p7.w;
    let min = min8(dp0, dp1, dp2, dp3, dp4, dp5, dp6, dp7);
    let max = max8(dp0, dp1, dp2, dp3, dp4, dp5, dp6, dp7);
    var vaabb = vec4<f32>(min.xy, max.xy);
    // convert ndc to texture coordinates by rescaling and flipping Y
    vaabb = vaabb.xwzy * vec4<f32>(0.5, -0.5, 0.5, -0.5) + 0.5;
    (*out).min = vec3<f32>(vaabb.xy, min.z);
    (*out).max = vec3<f32>(vaabb.zw, max.z);
    return true;
}

fn sample_hzb(smin: vec2<u32>, smax: vec2<u32>, mip: i32) -> f32 {
    let texel = vec4<u32>(0, 1, 2, 3);
    let sx = min(smin.x + texel, smax.xxxx);
    let sy = min(smin.y + texel, smax.yyyy);
    // TODO: switch to min samplers when wgpu has them
    // sampling 16 times a finer mip is worth the extra cost for better culling
    let a = sample_hzb_row(sx, sy.x, mip);
    let b = sample_hzb_row(sx, sy.y, mip);
    let c = sample_hzb_row(sx, sy.z, mip);
    let d = sample_hzb_row(sx, sy.w, mip);
    return min(min(a, b), min(c, d));
}

fn sample_hzb_row(sx: vec4<u32>, sy: u32, mip: i32) -> f32 {
    let a = textureLoad(depth_pyramid, vec2(sx.x, sy), mip).x;
    let b = textureLoad(depth_pyramid, vec2(sx.y, sy), mip).x;
    let c = textureLoad(depth_pyramid, vec2(sx.z, sy), mip).x;
    let d = textureLoad(depth_pyramid, vec2(sx.w, sy), mip).x;
    return min(min(a, b), min(c, d));
}

// TODO: We should probably be using a POT HZB texture?
fn occlusion_cull_screen_aabb(aabb: ScreenAabb, screen: vec2<f32>) -> bool {
    let hzb_size = ceil(screen * 0.5);
    let aabb_min = aabb.min.xy * hzb_size;
    let aabb_max = aabb.max.xy * hzb_size;

    let min_texel = vec2<u32>(max(aabb_min, vec2<f32>(0.0)));
    let max_texel = vec2<u32>(min(aabb_max, hzb_size - 1.0));
    let size = max_texel - min_texel;
    let max_size = max(size.x, size.y);

    // note: add 1 before max because the unsigned overflow behavior is intentional
    // it wraps around firstLeadingBit(0) = ~0 to 0
    // TODO: we actually sample a 4x4 block, so ideally this would be `max(..., 3u) - 3u`.
    // However, since our HZB is not a power of two, we need to be extra-conservative to not over-cull, so we go up a mip.
    var mip = max(firstLeadingBit(max_size) + 1u, 2u) - 2u;
    
    if any((max_texel >> vec2(mip)) > (min_texel >> vec2(mip)) + 3) {
        mip += 1u;
    }

    let smin = min_texel >> vec2<u32>(mip);
    let smax = max_texel >> vec2<u32>(mip);
    
    let curr_depth = sample_hzb(smin, smax, i32(mip));
    return aabb.max.z <= curr_depth;
}

fn occlusion_cull_projection() -> mat4x4<f32> {
#ifdef FIRST_CULLING_PASS
    return view.clip_from_world;
#else
    return previous_view.clip_from_world;
#endif
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
    let projection = occlusion_cull_projection();
    var near: f32;
    if projection[3][3] == 1.0 {
        near = projection[3][2] / projection[2][2];
    } else {
        near = projection[3][2];
    }

    let clip_from_local = occlusion_cull_clip_from_local(instance_id);
    var screen_aabb = ScreenAabb(vec3<f32>(0.0), vec3<f32>(0.0));
    if project_aabb(clip_from_local, near, aabb, &screen_aabb) {
        return occlusion_cull_screen_aabb(screen_aabb, view.viewport.zw);
    }
    return false;
}
