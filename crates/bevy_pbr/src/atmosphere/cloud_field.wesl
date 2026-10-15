// Coarse cloud signed-distance field (Nubis Cubed, Advances 2023, "SDF" / "Voxel Cloud
// Ray-Marcher"). Three steps:
//   init:  inside/outside of the coarse density (same predicate the raymarch uses to enter fine
//          mode) -> squared-distance seeds (rg) + vertical voxel size (b).
//   edt_x/edt_y/edt_z: separable exact Euclidean distance transform (Felzenszwalb & Huttenlocher
//          2012 lower-envelope form, brute-force within a window) for both the outside and the
//          inside distance. edt_z writes the final signed distance in metres (negative inside).

import package::atmosphere::bindings::{atmosphere, view};
import package::atmosphere::clouds::{
    cloud_layer,
    get_cloud_medium_density_coarse,
    cloud_field_origin,
    cloud_field_voxel_m,
    CLOUD_FIELD_RES_XY,
    CLOUD_FIELD_RES_Z,
};

@group(0) @binding(40) var out_field_init: texture_storage_3d<rgba16float, write>;
@group(0) @binding(41) var in_field: texture_3d<f32>;
@group(0) @binding(42) var out_field: texture_storage_3d<rgba16float, write>;
@group(0) @binding(43) var out_field_sdf: texture_storage_3d<rgba16float, write>;

// Squared distances are stored in xy-voxel units (fp16-safe: < 2048 within the windows).
const BIG: f32 = 60000.0;
// Same threshold as the raymarch's coarse -> fine switch (clouds_raymarch.wesl).
const INSIDE_EPS: f32 = 1e-4;
// Search windows (voxels): ~1.5 km horizontally, the whole layer vertically.
const WIN_XY: i32 = 8;
const WIN_Z: i32 = 31;

fn view_position() -> vec3<f32> {
    let p = (atmosphere.world_to_atmosphere * vec4(view.world_position, 1.0)).xyz;
    let r = length(p);
    let rmin = atmosphere.inner_radius + 1.0;
    return select(p, p / max(r, 1e-6) * rmin, r < rmin);
}

@compute @workgroup_size(8, 8, 1)
fn init(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= CLOUD_FIELD_RES_XY || id.y >= CLOUD_FIELD_RES_XY || id.z >= CLOUD_FIELD_RES_Z) {
        return;
    }
    let vox = cloud_field_voxel_m();
    let thick = cloud_layer.cloud_layer_end - cloud_layer.cloud_layer_start;
    let dz = thick / f32(CLOUD_FIELD_RES_Z);
    let xz = cloud_field_origin(view_position()) + (vec2<f32>(id.xy) + 0.5) * vox;
    let r = cloud_layer.cloud_layer_start + (f32(id.z) + 0.5) * dz;
    let p = vec3(xz.x, sqrt(max(r * r - dot(xz, xz), 0.0)), xz.y);
    let inside = get_cloud_medium_density_coarse(length(p), p) > INSIDE_EPS;
    // r: squared distance to the nearest inside voxel, g: to the nearest outside voxel.
    textureStore(out_field_init, id, vec4(select(BIG, 0.0, inside), select(0.0, BIG, inside), dz / vox, 0.0));
}

// One workgroup per grid line: the line is loaded into workgroup memory once, so each voxel's
// windowed min reads shared memory instead of 2*WIN+1 global texels (was ~5.7 ms, bandwidth bound).
var<workgroup> line: array<vec2<f32>, 256>;

fn edt_line(lane: u32, n: u32, win: i32, s: f32) -> vec2<f32> {
    var best = vec2(BIG);
    let c = i32(lane);
    for (var k = -win; k <= win; k++) {
        let j = c + k;
        if (j < 0 || j >= i32(n)) { continue; }
        let d = f32(k) * s;
        best = min(best, line[j] + vec2(d * d));
    }
    return best;
}

// Dispatch (1, RES_XY, RES_Z): lane = x.
@compute @workgroup_size(128, 1, 1)
fn edt_x(@builtin(local_invocation_index) lane: u32, @builtin(workgroup_id) wg: vec3<u32>) {
    let q = vec3<i32>(i32(lane), i32(wg.y), i32(wg.z));
    let v = textureLoad(in_field, q, 0);
    line[lane] = v.xy;
    workgroupBarrier();
    textureStore(out_field, q, vec4(edt_line(lane, CLOUD_FIELD_RES_XY, WIN_XY, 1.0), v.z, 0.0));
}

// Dispatch (RES_XY, 1, RES_Z): lane = y.
@compute @workgroup_size(128, 1, 1)
fn edt_y(@builtin(local_invocation_index) lane: u32, @builtin(workgroup_id) wg: vec3<u32>) {
    let q = vec3<i32>(i32(wg.x), i32(lane), i32(wg.z));
    let v = textureLoad(in_field, q, 0);
    line[lane] = v.xy;
    workgroupBarrier();
    textureStore(out_field, q, vec4(edt_line(lane, CLOUD_FIELD_RES_XY, WIN_XY, 1.0), v.z, 0.0));
}

// Dispatch (RES_XY, RES_XY, 1): lane = z.
@compute @workgroup_size(32, 1, 1)
fn edt_z(@builtin(local_invocation_index) lane: u32, @builtin(workgroup_id) wg: vec3<u32>) {
    let q = vec3<i32>(i32(wg.x), i32(wg.y), i32(lane));
    let v4 = textureLoad(in_field, q, 0);
    let dz = v4.z;
    line[lane] = v4.xy;
    workgroupBarrier();
    let v = edt_line(lane, CLOUD_FIELD_RES_Z, WIN_Z, dz) ;
    let vox = cloud_field_voxel_m();
    // Voxel centres -> surface: the boundary lies half a voxel between an inside and an outside
    // centre (smallest voxel axis).
    let half = 0.5 * min(1.0, dz);
    // Nothing found inside the window: the window radius is a safe lower bound.
    let d_out = min(sqrt(v.x), f32(WIN_XY));
    let d_in = min(sqrt(v.y), f32(WIN_XY));
    let sdf = select(d_out - half, half - d_in, v.x == 0.0) * vox;
    textureStore(out_field_sdf, q, vec4(sdf, 0.0, 0.0, 1.0));
}
