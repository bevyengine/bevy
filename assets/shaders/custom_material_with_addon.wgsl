struct Cursor { x: f32,  y: f32 };
@group(3) @binding(0)
var<uniform> cursor: Cursor;

@fragment
fn fragment(
    #import bevy_pbr::mesh_vertex_output
) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, cursor.x, cursor.y, 1.0);
}
