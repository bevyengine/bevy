#import bevy_render::view::View

@group(0) @binding(0) var<uniform> view: View;


struct LineGizmoUniform {
    line_width: f32,
    depth_bias: f32,
    resolution: u32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    // WebGL2 structs must be 16 byte aligned.
    _padding: f32,
#endif
}

@group(1) @binding(0) var<uniform> joints_gizmo: LineGizmoUniform;

struct VertexInput {
    @location(0) position_a: vec3<f32>,
    @location(1) position_b: vec3<f32>,
    @location(2) position_c: vec3<f32>,
    @location(3) color: vec4<f32>,
    @builtin(vertex_index) index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

const EPSILON: f32 = 4.88e-04;

@vertex
fn vertex_bevel(vertex: VertexInput) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2(0, 0),
        vec2(0, 0.5),
        vec2(0.5, 0),
    );
    var position = positions[vertex.index];

    var clip_a = view.clip_from_world * vec4(vertex.position_a, 1.);
    var clip_b = view.clip_from_world * vec4(vertex.position_b, 1.);
    var clip_c = view.clip_from_world * vec4(vertex.position_c, 1.);

    // Manual near plane clipping to avoid errors when doing the perspective divide inside this shader.
    clip_a = clip_near_plane(clip_a, clip_c);
    clip_b = clip_near_plane(clip_b, clip_a);
    clip_c = clip_near_plane(clip_c, clip_b);
    clip_a = clip_near_plane(clip_a, clip_c);

    let resolution = view.viewport.zw;
    let screen_a = resolution * (0.5 * clip_a.xy / clip_a.w + 0.5);
    let screen_b = resolution * (0.5 * clip_b.xy / clip_b.w + 0.5);
    let screen_c = resolution * (0.5 * clip_c.xy / clip_c.w + 0.5);

    var color = vertex.color;
    var line_width = joints_gizmo.line_width;

#ifdef PERSPECTIVE
    line_width /= clip_b.w;
#endif

    // Line thinness fade from https://acegikmo.com/shapes/docs/#anti-aliasing
    if line_width > 0.0 && line_width < 1. {
        color.a *= line_width;
        line_width = 1.;
    }

    let ab = normalize(screen_b - screen_a);
    let cb = normalize(screen_b - screen_c);
    let ab_norm = vec2(-ab.y, ab.x);
    let cb_norm = vec2(cb.y, -cb.x);
    let tangent = normalize(ab - cb);
    let normal = vec2(-tangent.y, tangent.x);
    let sigma = sign(dot(ab + cb, normal));

    var p0 = line_width * sigma * ab_norm;
    var p1 = line_width * sigma * cb_norm;

    let screen = screen_b + position.x * p0 + position.y * p1;

    let depth = depth(clip_b);

    var clip_position = vec4(clip_b.w * ((2. * screen) / resolution - 1.), depth, clip_b.w);
    return VertexOutput(clip_position, color);
}

@vertex
fn vertex_miter(vertex: VertexInput) -> VertexOutput {
    var positions = array<vec3<f32>, 6>(
        vec3(0, 0, 0),
        vec3(0.5, 0, 0),
        vec3(0, 0.5, 0),
        vec3(0, 0, 0),
        vec3(0, 0.5, 0),
        vec3(0, 0, 0.5),
    );
    var position = positions[vertex.index];

    var clip_a = view.clip_from_world * vec4(vertex.position_a, 1.);
    var clip_b = view.clip_from_world * vec4(vertex.position_b, 1.);
    var clip_c = view.clip_from_world * vec4(vertex.position_c, 1.);

    // Manual near plane clipping to avoid errors when doing the perspective divide inside this shader.
    clip_a = clip_near_plane(clip_a, clip_c);
    clip_b = clip_near_plane(clip_b, clip_a);
    clip_c = clip_near_plane(clip_c, clip_b);
    clip_a = clip_near_plane(clip_a, clip_c);

    let resolution = view.viewport.zw;
    let screen_a = resolution * (0.5 * clip_a.xy / clip_a.w + 0.5);
    let screen_b = resolution * (0.5 * clip_b.xy / clip_b.w + 0.5);
    let screen_c = resolution * (0.5 * clip_c.xy / clip_c.w + 0.5);

    var color = vertex.color;
    var line_width = joints_gizmo.line_width;

#ifdef PERSPECTIVE
    line_width /= clip_b.w;
#endif

    // Line thinness fade from https://acegikmo.com/shapes/docs/#anti-aliasing
    if line_width > 0.0 && line_width < 1. {
        color.a *= line_width;
        line_width = 1.;
    }

    let ab = normalize(screen_b - screen_a);
    let cb = normalize(screen_b - screen_c);
    let ab_norm = vec2(-ab.y, ab.x);
    let cb_norm = vec2(cb.y, -cb.x);
    let tangent = normalize(ab - cb);
    let normal = vec2(-tangent.y, tangent.x);
    let sigma = sign(dot(ab + cb, normal));

    var p0 = line_width * sigma * ab_norm;
    var p1 = line_width * sigma * normal / dot(normal, ab_norm);
    var p2 = line_width * sigma * cb_norm;
    
    var screen = screen_b + position.x * p0 + position.y * p1 + position.z * p2;

    var depth = depth(clip_b);

    var clip_position = vec4(clip_b.w * ((2. * screen) / resolution - 1.), depth, clip_b.w);
    return VertexOutput(clip_position, color);
}

@vertex
fn vertex_round(vertex: VertexInput) -> VertexOutput {
    var clip_a = view.clip_from_world * vec4(vertex.position_a, 1.);
    var clip_b = view.clip_from_world * vec4(vertex.position_b, 1.);
    var clip_c = view.clip_from_world * vec4(vertex.position_c, 1.);

    // Manual near plane clipping to avoid errors when doing the perspective divide inside this shader.
    clip_a = clip_near_plane(clip_a, clip_c);
    clip_b = clip_near_plane(clip_b, clip_a);
    clip_c = clip_near_plane(clip_c, clip_b);
    clip_a = clip_near_plane(clip_a, clip_c);

    let resolution = view.viewport.zw;
    let screen_a = resolution * (0.5 * clip_a.xy / clip_a.w + 0.5);
    let screen_b = resolution * (0.5 * clip_b.xy / clip_b.w + 0.5);
    let screen_c = resolution * (0.5 * clip_c.xy / clip_c.w + 0.5);

    var color = vertex.color;
    var line_width = joints_gizmo.line_width;

#ifdef PERSPECTIVE
    line_width /= clip_b.w;
#endif

    // Line thinness fade from https://acegikmo.com/shapes/docs/#anti-aliasing
    if line_width > 0.0 && line_width < 1. {
        color.a *= line_width;
        line_width = 1.;
    }

    let ab = normalize(screen_b - screen_a);
    let cb = normalize(screen_b - screen_c);
    let ab_norm = vec2(-ab.y, ab.x);
    let cb_norm = vec2(cb.y, -cb.x);

    // We render `joints_gizmo.resolution`triangles. The vertices in each triangle are ordered as follows:
    // - 0: The 'center' vertex at `screen_b`.
    // - 1: The vertex closer to the ab line.
    // - 2: The vertex closer to the cb line. 
    var in_triangle_index = f32(vertex.index) % 3.0;
    var tri_index = floor(f32(vertex.index) / 3.0);
    var radius = sign(in_triangle_index) * 0.5 * line_width;
    var theta = acos(dot(ab_norm, cb_norm));
    let sigma = sign(dot(ab_norm, cb));
    var angle = theta * (tri_index + in_triangle_index - 1) / f32(joints_gizmo.resolution);
    var position_x = sigma * radius * cos(angle);
    var position_y = radius * sin(angle);

    var screen = screen_b + position_x * ab_norm + position_y * ab;

    var depth = depth(clip_b);

    var clip_position = vec4(clip_b.w * ((2. * screen) / resolution - 1.), depth, clip_b.w);
    return VertexOutput(clip_position, color);
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

fn depth(clip: vec4<f32>) -> f32 {
    var depth: f32;
    if joints_gizmo.depth_bias >= 0. {
        depth = clip.z * (1. - joints_gizmo.depth_bias);
    } else {
        // depth * (clip.w / depth)^-depth_bias. So that when -depth_bias is 1.0, this is equal to clip.w
        // and when equal to 0.0, it is exactly equal to depth.
        // the epsilon is here to prevent the depth from exceeding clip.w when -depth_bias = 1.0
        // clip.w represents the near plane in homogeneous clip space in bevy, having a depth
        // of this value means nothing can be in front of this
        // The reason this uses an exponential function is that it makes it much easier for the
        // user to chose a value that is convenient for them
        depth = clip.z * exp2(-joints_gizmo.depth_bias * log2(clip.w / clip.z - EPSILON));
    }
    return depth;
}

struct FragmentInput {
    @location(0) color: vec4<f32>,
};

struct FragmentOutput {
    @location(0) color: vec4<f32>,
};

@fragment
fn fragment(in: FragmentInput) -> FragmentOutput {
    // return FragmentOutput(vec4(1, 1, 1, 1));
    return FragmentOutput(in.color);
}
