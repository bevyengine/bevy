#ifdef GIZMO_3D
    #import bevy_pbr::mesh_view_bindings
    #import bevy_pbr::mesh_types

    @group(1) @binding(0)
    var<uniform> mesh: Mesh;

    #import bevy_pbr::mesh_functions
#else
    #import bevy_sprite::mesh2d_view_bindings
    #import bevy_sprite::mesh2d_types

    @group(1) @binding(0)
    var<uniform> mesh: Mesh2d;

    #import bevy_sprite::mesh2d_functions
#endif

struct Gizmo {
    color: vec4<f32>,
}

@group(2) @binding(0)
var<uniform> gizmo: Gizmo;

struct VertexInput {
    @location(0) pos: vec3<f32>,
#ifdef VERTEX_COLORS
    @location(1) color: vec4<f32>,
#endif
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
#ifdef VERTEX_COLORS
    @location(0) color: vec4<f32>,
#endif
}

struct FragmentOutput {
#ifdef GIZMO_3D
    @builtin(frag_depth) depth: f32,
#endif
    @location(0) color: vec4<f32>,
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

#ifdef GIZMO_3D
    let world_pos = mesh_position_local_to_world(mesh.model, vec4<f32>(in.pos, 1.0));
    out.pos = mesh_position_world_to_clip(world_pos);
#else
    let world_pos = mesh2d_position_local_to_world(mesh.model, vec4<f32>(in.pos, 1.0));
    out.pos = mesh2d_position_world_to_clip(world_pos);
#endif

#ifdef VERTEX_COLORS
    out.color = in.color;
#endif

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

#ifdef GIZMO_3D
#ifdef DEPTH_TEST
    out.depth = in.pos.z;
#else
    out.depth = 1.0;
#endif
#endif

#ifdef VERTEX_COLORS
    out.color = in.color * gizmo.color;
#else
    out.color = gizmo.color;
#endif

    return out;
}
