// Visual regression test for `Mesh2d` tangent space. Red means a vector is not unit length.
import bevy_sprite_render::mesh2d::vertex_output::VertexOutput;

fn tangent_space_color(v: vec3<f32>) -> vec3<f32> {
    // Checked before remapping, because an over-long vector remaps to the same color as a unit one.
    // Loose enough to absorb interpolation and f32 error.
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
    // Handedness.
    if mesh.world_tangent.w > 0.0 {
        return vec4<f32>(0.0, 0.4, 1.0, 1.0);
    }
    return vec4<f32>(1.0, 0.9, 0.0, 1.0);
}
