import bevy_sprite_render::mesh2d::vertex_output::VertexOutput;

// Visualizes the interpolated tangent space of a `Mesh2d`. A correctly transformed quad facing
// the camera is lavender on top and salmon in the middle; mirroring it flips the tangent band to
// teal and the handedness band to yellow.
//
// The length is checked before the direction is remapped, because remapping alone cannot detect
// an over-long vector: it saturates to the same color a unit vector produces.
fn tangent_space_color(v: vec3<f32>) -> vec3<f32> {
    // Loose enough to absorb interpolation and f32 error, and two orders of magnitude tighter
    // than the smallest regression this guards against.
    if abs(length(v) - 1.0) > 0.01 {
        return vec3<f32>(1.0, 0.0, 0.0);
    }
    return v * 0.5 + 0.5;
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    if mesh.uv.y < 0.4 {
        return vec4<f32>(tangent_space_color(mesh.world_normal), 1.0);
    }
    if mesh.uv.y < 0.8 {
        return vec4<f32>(tangent_space_color(mesh.world_tangent.xyz), 1.0);
    }
    // Handedness: blue where `w` is positive, yellow where it is negative.
    if mesh.world_tangent.w > 0.0 {
        return vec4<f32>(0.0, 0.4, 1.0, 1.0);
    }
    return vec4<f32>(1.0, 0.9, 0.0, 1.0);
}
