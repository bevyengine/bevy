// Raymarches a signed distance field and packs it into the deferred gbuffer exactly as
// a `StandardMaterial` would (via `deferred_gbuffer_from_pbr_input`), so Bevy's deferred
// PBR lighting shades it like any other geometry. This example assumes familiarity with
// raymarching.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::view_transformations::{
    uv_to_ndc,
    position_ndc_to_view,
    position_ndc_to_world,
    direction_view_to_world,
    position_world_to_view,
    view_z_to_depth_ndc,
}
#import bevy_pbr::pbr_deferred_functions::deferred_gbuffer_from_pbr_input
#import bevy_pbr::pbr_types::{pbr_input_new, STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE}
#import bevy_pbr::mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT
#import bevy_render::globals::Globals

@group(0) @binding(1) var<uniform> globals: Globals;

const FAR: f32 = 100.0;
const SURFACE_EPSILON: f32 = 0.001;
const MAX_STEPS: u32 = 160u;
const STEP_SCALE: f32 = 0.5;

// Swap this for any SDF you like, the gbuffer integration below is independent of the
// shape.
const GYROID_SCALE: f32 = 5.0;

fn gyroid_field(p: vec3<f32>) -> f32 {
    let q = p * GYROID_SCALE + globals.time * 0.6;
    return dot(sin(q), cos(q.yzx));
}

fn map(p: vec3<f32>) -> f32 {
    let shell = (abs(gyroid_field(p)) - 0.14) / GYROID_SCALE;
    let ball = length(p) - 1.3;
    return max(ball, shell);
}

fn sdf_normal(p: vec3<f32>) -> vec3<f32> {
    let e = vec2<f32>(1.0, -1.0) * 0.0005;
    return normalize(
        e.xyy * map(p + e.xyy) +
        e.yyx * map(p + e.yyx) +
        e.yxy * map(p + e.yxy) +
        e.xxx * map(p + e.xxx)
    );
}

fn raymarch(ray_origin: vec3<f32>, ray_dir: vec3<f32>) -> f32 {
    var t = 0.0;
    for (var i = 0u; i < MAX_STEPS; i++) {
        let d = map(ray_origin + ray_dir * t);
        if d < SURFACE_EPSILON {
            return t;
        }
        t += d * STEP_SCALE;
        if t > FAR {
            break;
        }
    }
    return FAR;
}

struct Ray {
    origin: vec3<f32>,
    dir: vec3<f32>,
}

fn ray_for_uv(uv: vec2<f32>) -> Ray {
    let ndc = uv_to_ndc(uv);
    var ray: Ray;
    // Check ortho projection
    if view.clip_from_view[3].w == 1.0 {
        ray.origin = position_ndc_to_world(vec3<f32>(ndc, 1.0));
        ray.dir = normalize(direction_view_to_world(vec3<f32>(0.0, 0.0, -1.0)));
    } else {
        ray.origin = view.world_position;
        ray.dir = normalize(direction_view_to_world(position_ndc_to_view(vec3<f32>(ndc, 1.0))));
    }
    return ray;
}

fn depth_for_world_pos(world_pos: vec3<f32>) -> f32 {
    return view_z_to_depth_ndc(position_world_to_view(world_pos).z);
}

// Our render pass has two color targets, matching the deferred prepass:
//   location 0: the packed gbuffer (Rgba32Uint)
//   location 1: the deferred lighting pass id (R8Uint)
struct GBufferOutput {
    @location(0) deferred: vec4<u32>,
    @location(1) deferred_lighting_pass_id: u32,
    @builtin(frag_depth) depth: f32,
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> GBufferOutput {
    let ray = ray_for_uv(in.uv);
    let t = raymarch(ray.origin, ray.dir);

    // discard so we leave the mesh gbuffer intact when we miss 
    if t >= FAR {
        discard;
    }

    let world_pos = ray.origin + ray.dir * t;
    let normal = sdf_normal(world_pos);

    // Fill in a PbrInput as a StandardMaterial fragment would
    var pbr_input = pbr_input_new();
    pbr_input.frag_coord = vec4<f32>(in.position.xy, depth_for_world_pos(world_pos), 1.0);
    pbr_input.world_position = vec4<f32>(world_pos, 1.0);
    pbr_input.world_normal = normal;
    pbr_input.N = normal;
    pbr_input.V = normalize(view.world_position - world_pos);

    // Per-pixel base color
    let field = gyroid_field(world_pos);
    let color = 0.5 + 0.5 * cos(
        6.2831 * (field * 0.4 + globals.time * 0.05) + vec3<f32>(0.0, 0.8, 1.6)
    );
    pbr_input.material.base_color = vec4<f32>(color, 1.0);
    pbr_input.material.metallic = 0.7;
    pbr_input.material.perceptual_roughness = 0.25;
    pbr_input.material.flags = STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE;
    // Let the surface receive shadows cast by other geometry.
    pbr_input.flags = MESH_FLAGS_SHADOW_RECEIVER_BIT;

    var out: GBufferOutput;
    out.deferred = deferred_gbuffer_from_pbr_input(pbr_input);
    // The lighting pass reads this per-pixel id to choose which lighting shader runs.
    //  1 is Bevy's built-in PBR deferred shader.
    out.deferred_lighting_pass_id = 1u;
    out.depth = pbr_input.frag_coord.z;
    return out;
}

// When rendering into a light's shadow map we only need depth
@fragment
fn fragment_shadow(in: FullscreenVertexOutput) -> @builtin(frag_depth) f32 {
    let ray = ray_for_uv(in.uv);
    let t = raymarch(ray.origin, ray.dir);
    if t >= FAR {
        discard;
    }
    return depth_for_world_pos(ray.origin + ray.dir * t);
}
