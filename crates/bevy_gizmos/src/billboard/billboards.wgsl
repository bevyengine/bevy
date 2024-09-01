// TODO use common view binding
#import bevy_render::view::View

@group(0) @binding(0) var<uniform> view: View;


struct BillboardGizmoUniform {
    size: f32,
    depth_bias: f32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    // WebGL2 structs must be 16 byte aligned.
    _padding: vec2<f32>,
#endif
}

@group(1) @binding(0) var<uniform> billboard_gizmo: BillboardGizmoUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @builtin(vertex_index) index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

const EPSILON: f32 = 4.88e-04;

@vertex
fn vertex(vertex: VertexInput) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2(-0.5, -0.5),
        vec2(-0.5, 0.5),
        vec2(0.5, 0.5),
        vec2(-0.5, -0.5),
        vec2(0.5, 0.5),
        vec2(0.5, -0.5)
    );
    var offset = positions[vertex.index];

	let center_clip = view.clip_from_world * vec4(vertex.position, 1.0);

    let resolution = view.viewport.zw;
    let screen_center = resolution * (0.5 * center_clip.xy / center_clip.w + 0.5);

    var color = vertex.color;
    var billboard_size = billboard_gizmo.size;
#ifdef PERSPECTIVE
    billboard_size /= center_clip.w;
#endif
    if billboard_size < 1. {
        color.a *= billboard_size;
        billboard_size = 1.;
    }

    let screen = screen_center + offset * billboard_size;

    var depth: f32;
    if billboard_gizmo.depth_bias >= 0. {
        depth = center_clip.z * (1. - billboard_gizmo.depth_bias);
    } else {
        // depth * (center_clip.w / depth)^-depth_bias. So that when -depth_bias is 1.0, this is equal to center_clip.w
        // and when equal to 0.0, it is exactly equal to depth.
        // the epsilon is here to prevent the depth from exceeding center_clip.w when -depth_bias = 1.0
        // center_clip.w represents the near plane in homogeneous clip space in bevy, having a depth
        // of this value means nothing can be in front of this
        // The reason this uses an exponential function is that it makes it much easier for the
        // user to chose a value that is convenient for them
        depth = center_clip.z * exp2(-billboard_gizmo.depth_bias * log2(center_clip.w / center_clip.z - EPSILON));
    }

    var clip_position = vec4(center_clip.w * ((2. * screen) / resolution - 1.), depth, center_clip.w);

    var result: VertexOutput;
	result.clip_position = clip_position;
	result.color = color;
	return result;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
};

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    return FragmentOutput(in.color);
}