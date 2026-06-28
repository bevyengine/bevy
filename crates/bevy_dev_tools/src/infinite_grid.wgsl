#import bevy_render::view::View
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct InfiniteGridUniform {
    rot_matrix: mat3x3<f32>,
    origin: vec3<f32>,
    normal: vec3<f32>,
};

struct InfiniteGridSettings {
    scale: f32,
    // 1 / fadeout_distance
    one_over_fadeout_distance: f32,
    // 1 / dot_fadeout_strength
    one_over_dot_fadeout: f32,
    x_axis_col: vec3<f32>,
    z_axis_col: vec3<f32>,
    minor_line_col: vec4<f32>,
    major_line_col: vec4<f32>,
};

@group(0) @binding(0) var<uniform> view: View;

@group(1) @binding(0) var<uniform> infinite_grid: InfiniteGridUniform;
@group(1) @binding(1) var<uniform> grid_settings: InfiniteGridSettings;

// Same as view_transformations::position_ndc_to_world but we can't use it since
// it relies on bevy's view bind group
fn position_ndc_to_world(ndc_pos: vec3<f32>) -> vec3<f32> {
    let world_pos = view.world_from_clip * vec4(ndc_pos, 1.0);
    return world_pos.xyz / world_pos.w;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

@fragment
fn fragment(in: FullscreenVertexOutput) -> FragmentOutput {
    // Reconstruct clip-space XY from UV, then unproject to world space
    let clip_xy = in.uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
    let near_point = position_ndc_to_world(vec3(clip_xy, 1.0));
    let far_point = position_ndc_to_world(vec3(clip_xy, 0.001));

    // Cast a ray from the near plane towards the far plane
    let ray_origin = near_point;
    let ray_direction = normalize(far_point - near_point);
    let plane_normal = infinite_grid.normal;
    let plane_origin = infinite_grid.origin;

    // Ray-plane intersection
    // t is the signed distance to the plane
    let point_to_point = plane_origin - ray_origin;
    let t = dot(plane_normal, point_to_point) / dot(ray_direction, plane_normal);
    let frag_pos_3d = ray_direction * t + ray_origin;

    // Project the 3D hit point into the grid's local 2D coordinate space so
    // that grid lines are always axis-aligned regardless of the grid's rotation.
    let planar_offset = frag_pos_3d - plane_origin;
    let rotation_matrix = infinite_grid.rot_matrix;
    let plane_coords = (rotation_matrix * planar_offset).xz;

    // Compute the clip-space depth of the hit point to make the lines opaque
    let view_space_pos = view.view_from_world * vec4(frag_pos_3d, 1.);
    let clip_space_pos = view.clip_from_view * view_space_pos;
    let clip_depth = clip_space_pos.z / clip_space_pos.w;
    let real_depth = -view_space_pos.z;

    var out: FragmentOutput;
    out.depth = clip_depth;

    // Scale the plane coordinates to control the size of the grid
    let scale = grid_settings.scale;
    let coord = plane_coords * scale;

    // Compute minor lines
    let derivative = fwidth(coord);
    let grid = abs(fract(coord - 0.5) - 0.5) / derivative;
    let minor_line = min(grid.x, grid.y);

    // Compute major lines
    let derivative2 = fwidth(coord * 0.1);
    let grid2 = abs(fract((coord * 0.1) - 0.5) - 0.5) / derivative2;
    let major_line = min(grid2.x, grid2.y);

    // Compute axis lines
    let grid3 = abs(coord) / derivative;
    let axis_line = min(grid3.x, grid3.y);

    // Compute the alpha based on the priority of the lines, axis > major > minor
    var alpha = vec3(1.0) - min(vec3(axis_line, major_line, minor_line), vec3(1.0));
    alpha.y *= (1.0 - alpha.x) * grid_settings.major_line_col.a;
    alpha.z *= (1.0 - (alpha.x + alpha.y)) * grid_settings.minor_line_col.a;

    // Fade the grid linearly with distance from the camera.
    let dist_fadeout = min(1., 1. - grid_settings.one_over_fadeout_distance * real_depth);
    let dot_fadeout = abs(dot(infinite_grid.normal, normalize(view.world_position - frag_pos_3d)));
    let alpha_fadeout = mix(dist_fadeout, 1., dot_fadeout) 
        * min(grid_settings.one_over_dot_fadeout * dot_fadeout, 1.);

    // Normalize the alpha
    let a_0 = alpha.x + alpha.y + alpha.z;
    alpha /= a_0;
    // In case a_0 is 0 we need to clamp the result to avoid NaNs
    alpha = clamp(alpha, vec3(0.0), vec3(1.0));

    // Choose the axis color based on which axis this fragment is closest to
    let axis_color = mix(
        grid_settings.x_axis_col, 
        grid_settings.z_axis_col, 
        step(grid3.x, grid3.y)
    );

    var grid_color = vec4(
        axis_color * alpha.x 
            + grid_settings.major_line_col.rgb * alpha.y 
            + grid_settings.minor_line_col.rgb * alpha.z,
        max(a_0 * alpha_fadeout, 0.0),
    );
    out.color = grid_color;

    return out;
}

