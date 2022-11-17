// read-only storage buffer
@group(1) @binding(0)
var<storage, read> color_buffer: array<f32>;

// read-write storage buffer
@group(1) @binding(1)
var<storage, read_write> writable_buffer: array<f32>;

@fragment
fn fragment(
    #import bevy_pbr::mesh_vertex_output
) -> @location(0) vec4<f32> {
    writable_buffer[0] += 10.;
    writable_buffer[1] += 5.;
    writable_buffer[2] += 2.;
    writable_buffer[3] += 1.;

    return vec4(color_buffer[0], color_buffer[1], color_buffer[2], color_buffer[3]);
}
