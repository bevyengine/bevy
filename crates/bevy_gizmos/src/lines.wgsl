// TODO use common view binding
#import bevy_render::view::View

@group(0) @binding(0) var<uniform> view: View;


struct LineGizmoUniform {
    line_width: f32,
    depth_bias: f32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    // WebGL2 structs must be 16 byte aligned.
    _padding: vec2<f32>,
#endif
}

@group(1) @binding(0) var<uniform> line_gizmo: LineGizmoUniform;

struct VertexInput {
    @location(0) position_a: vec3<f32>,
    @location(1) position_b: vec3<f32>,
    @location(2) color_a: vec4<f32>,
    @location(3) color_b: vec4<f32>,
    @builtin(vertex_index) index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: f32,
};

const EPSILON: f32 = 4.88e-04;

@vertex
fn vertex(vertex: VertexInput) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2(-0.5, 0.),
        vec2(-0.5, 1.),
        vec2(0.5, 1.),
        vec2(-0.5, 0.),
        vec2(0.5, 1.),
        vec2(0.5, 0.)
    );
    let position = positions[vertex.index];

    // algorithm based on https://wwwtyro.net/2019/11/18/instanced-lines.html
    var clip_a = view.clip_from_world * vec4(vertex.position_a, 1.);
    var clip_b = view.clip_from_world * vec4(vertex.position_b, 1.);

    // Manual near plane clipping to avoid errors when doing the perspective divide inside this shader.
    clip_a = clip_near_plane(clip_a, clip_b);
    clip_b = clip_near_plane(clip_b, clip_a);
    let clip = mix(clip_a, clip_b, position.y);

    let resolution = view.viewport.zw;
    let screen_a = resolution * (0.5 * clip_a.xy / clip_a.w + 0.5);
    let screen_b = resolution * (0.5 * clip_b.xy / clip_b.w + 0.5);

    let y_basis = normalize(screen_b - screen_a);
    let x_basis = vec2(-y_basis.y, y_basis.x);

    var color = mix(vertex.color_a, vertex.color_b, position.y);

    var line_width = line_gizmo.line_width;
    var alpha = 1.;

    var uv: f32;
#ifdef PERSPECTIVE
    line_width /= clip.w;

    // get height of near clipping plane in world space
    let pos0 = view.view_from_clip * vec4(0, -1, 0, 1); // Bottom of the screen
    let pos1 = view.view_from_clip * vec4(0, 1, 0, 1); // Top of the screen
    let near_clipping_plane_height = length(pos0.xyz - pos1.xyz);

    // We can't use vertex.position_X because we may have changed the clip positions with clip_near_plane
    let position_a = view.world_from_clip * clip_a;
    let position_b = view.world_from_clip * clip_b;
    let world_distance = length(position_a.xyz - position_b.xyz);

    // Offset to compensate for moved clip positions. If removed dots on lines will slide when position a is ofscreen.
    let clipped_offset = length(position_a.xyz - vertex.position_a);

    uv = (clipped_offset + position.y * world_distance) * resolution.y / near_clipping_plane_height / line_gizmo.line_width;
#else
    // Get the distance of b to the camera along camera axes
    let camera_b = view.view_from_clip * clip_b;

    // This differentiates between orthographic and perspective cameras.
    // For orthographic cameras no depth adaptment (depth_adaptment = 1) is needed.
    var depth_adaptment: f32;
    if (clip_b.w == 1.0) {
        depth_adaptment = 1.0;
    }
    else {
        depth_adaptment = -camera_b.z;
    }
    uv = position.y * depth_adaptment * length(screen_b - screen_a) / line_gizmo.line_width;
#endif

    // Line thinness fade from https://acegikmo.com/shapes/docs/#anti-aliasing
    if line_width > 0.0 && line_width < 1. {
        color.a *= line_width;
        line_width = 1.;
    }

    let x_offset = line_width * position.x * x_basis;
    let screen = mix(screen_a, screen_b, position.y) + x_offset;

    var depth: f32;
    if line_gizmo.depth_bias >= 0. {
        depth = clip.z * (1. - line_gizmo.depth_bias);
    } else {
        // depth * (clip.w / depth)^-depth_bias. So that when -depth_bias is 1.0, this is equal to clip.w
        // and when equal to 0.0, it is exactly equal to depth.
        // the epsilon is here to prevent the depth from exceeding clip.w when -depth_bias = 1.0
        // clip.w represents the near plane in homogeneous clip space in bevy, having a depth
        // of this value means nothing can be in front of this
        // The reason this uses an exponential function is that it makes it much easier for the
        // user to chose a value that is convenient for them
        depth = clip.z * exp2(-line_gizmo.depth_bias * log2(clip.w / clip.z - EPSILON));
    }

    var clip_position = vec4(clip.w * ((2. * screen) / resolution - 1.), depth, clip.w);

    return VertexOutput(clip_position, color, uv);
}

fn clip_near_plane(a: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    // Move a if a is behind the near plane and b is in front. 
    if a.z > a.w && b.z <= b.w {
        // Interpolate a towards b until it's at the near plane.
        let distance_a = a.z - a.w;
        let distance_b = b.z - b.w;
        // Add an epsilon to the interpolator to ensure that the point is
        // not just behind the clip plane due to floating-point imprecision.
        let t = distance_a / (distance_a - distance_b) + EPSILON;
        return mix(a, b, t);
    }
    return a;
}

struct FragmentInput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: f32,
};

struct FragmentOutput {
    @location(0) color: vec4<f32>,
};

@fragment
fn fragment_solid(in: FragmentInput) -> FragmentOutput {
    return FragmentOutput(in.color);
}
@fragment
fn fragment_dotted(in: FragmentInput) -> FragmentOutput {
    var alpha: f32;
#ifdef PERSPECTIVE
    alpha = 1 - floor(in.uv % 2.0);
#else
    alpha = 1 - floor((in.uv * in.position.w) % 2.0);
#endif
    
    return FragmentOutput(vec4(in.color.xyz, in.color.w * alpha));
}
