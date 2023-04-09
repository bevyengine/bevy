// TODO use common view binding
#from bevy_render::view import View

@group(0) @binding(0)
var<uniform> view: ::View;


struct VertexInput {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
}

struct FragmentOutput {
#ifdef GIZMO_LINES_3D
    @builtin(frag_depth) depth: f32,
#endif
    @location(0) color: vec4<f32>,
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.pos = view.view_proj * vec4<f32>(in.pos, 1.0);
    out.color = in.color;

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

#ifdef GIZMO_LINES_3D
#ifdef DEPTH_TEST
    out.depth = in.pos.z;
#else
    out.depth = 1.0;
#endif
#endif

    out.color = in.color;
    return out;
}
