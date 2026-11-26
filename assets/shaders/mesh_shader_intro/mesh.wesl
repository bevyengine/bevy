// these functions cause additional bindings to be brought in, which
// fails the shader compilations. We should improve the ability to import these
// in the future and then use the imports here.
// import bevy_pbr::render::view_transformations::position_world_to_clip;
// import bevy_pbr::render::mesh_functions::mesh_position_local_to_world;
import super::{
    bindings::{globals, view, VertexOutput, TaskPayload},
};

enable wgpu_mesh_shader;

fn position_world_to_clip(world_pos: vec3<f32>) -> vec4<f32> {
    let clip_pos = view.clip_from_world * vec4(world_pos, 1.0);
    return clip_pos;
}

fn mesh_position_local_to_world(world_from_local: mat4x4<f32>, vertex_position: vec4<f32>) -> vec4<f32> {
    return world_from_local * vertex_position;
}

// This data represents a cube, for simplicity reasons it is hardcoded.
// half_size is half the size of the cube
const half_size = vec3(0.1);
const min = -half_size;
const max = half_size;

// Suppose Y-up right hand, and camera look from +Z to -Z
// 192 items
const vertices = array(
    // xyz, normal.xyz, uv.xy
    // Front
    min.x, min.y, max.z, 0.0, 0.0, 1.0, 0.0, 0.0,
    max.x, min.y, max.z, 0.0, 0.0, 1.0, 1.0, 0.0,
    max.x, max.y, max.z, 0.0, 0.0, 1.0, 1.0, 1.0,
    min.x, max.y, max.z, 0.0, 0.0, 1.0, 0.0, 1.0,
    // Back
    min.x, max.y, min.z, 0.0, 0.0, -1.0, 1.0, 0.0,
    max.x, max.y, min.z, 0.0, 0.0, -1.0, 0.0, 0.0,
    max.x, min.y, min.z, 0.0, 0.0, -1.0, 0.0, 1.0,
    min.x, min.y, min.z, 0.0, 0.0, -1.0, 1.0, 1.0,
    // Right
    max.x, min.y, min.z, 1.0, 0.0, 0.0, 0.0, 0.0,
    max.x, max.y, min.z, 1.0, 0.0, 0.0, 1.0, 0.0,
    max.x, max.y, max.z, 1.0, 0.0, 0.0, 1.0, 1.0,
    max.x, min.y, max.z, 1.0, 0.0, 0.0, 0.0, 1.0,
    // Left
    min.x, min.y, max.z, -1.0, 0.0, 0.0, 1.0, 0.0,
    min.x, max.y, max.z, -1.0, 0.0, 0.0, 0.0, 0.0,
    min.x, max.y, min.z, -1.0, 0.0, 0.0, 0.0, 1.0,
    min.x, min.y, min.z, -1.0, 0.0, 0.0, 1.0, 1.0,
    // Top
    max.x, max.y, min.z, 0.0, 1.0, 0.0, 1.0, 0.0,
    min.x, max.y, min.z, 0.0, 1.0, 0.0, 0.0, 0.0,
    min.x, max.y, max.z, 0.0, 1.0, 0.0, 0.0, 1.0,
    max.x, max.y, max.z, 0.0, 1.0, 0.0, 1.0, 1.0,
    // Bottom
    max.x, min.y, max.z, 0.0, -1.0, 0.0, 0.0, 0.0,
    min.x, min.y, max.z, 0.0, -1.0, 0.0, 1.0, 0.0,
    min.x, min.y, min.z, 0.0, -1.0, 0.0, 1.0, 1.0,
    max.x, min.y, min.z, 0.0, -1.0, 0.0, 0.0, 1.0
);

// 36 items
const indices = array(
    0, 1, 2, 2, 3, 0, // front
    4, 5, 6, 6, 7, 4, // back
    8, 9, 10, 10, 11, 8, // right
    12, 13, 14, 14, 15, 12, // left
    16, 17, 18, 18, 19, 16, // top
    20, 21, 22, 22, 23, 20, // bottom
);


struct PrimitiveOutput {
    @builtin(triangle_indices) indices: vec3<u32>,
    @builtin(cull_primitive) cull: bool,
    @per_primitive @location(3) colorMask: vec4<f32>,
}

var<task_payload> taskPayload: TaskPayload;

struct MeshOutput {
    @builtin(vertices) vertices: array<VertexOutput, 24>,
    @builtin(primitives) primitives: array<PrimitiveOutput, 12>,
    @builtin(vertex_count) vertex_count: u32,
    @builtin(primitive_count) primitive_count: u32,
}

var<workgroup> mesh_output: MeshOutput;

struct MeshInput {
    @builtin(global_invocation_id) global_invocation_id: vec3u,
    @builtin(num_workgroups) num_workgroups: vec3u,
    @builtin(workgroup_id) workgroup_id: vec3u
}

@mesh(mesh_output)
@payload(taskPayload)
@workgroup_size(1)
fn mesh(mesh_input: MeshInput) {
    mesh_output.vertex_count = 24;
    mesh_output.primitive_count = 12;
    
    // we "fake" the `world_from_local` matrix that normally exists for
    // meshes because we're generating and positioning cubes in arbitrary
    // positions. The global_invocation_id will give us an x/y/z for every
    // point in the 3d space we're filling.
    let world_from_local = mat4x4(
        vec4(1., 0., 0., 0,),
        vec4(0., 1., 0., 0.),
        vec4(0., 0., 1., 0.),
        vec4(
            f32(mesh_input.global_invocation_id.x),
            f32(mesh_input.global_invocation_id.y),
            f32(mesh_input.global_invocation_id.z),
            1.,
        ),
    );

    // Set the data for each vertex, including position, normal, and uv.
    for (var i: i32 = 0; i < 24; i++) {
        let offset = i * 8;
        mesh_output.vertices[i].position =  position_world_to_clip(
           mesh_position_local_to_world(world_from_local, 
                vec4(vertices[offset] , vertices[offset + 1], vertices[offset + 2], 1.)
           ).xyz
       );
        mesh_output.vertices[i].normal = vec3(vertices[offset + 3], vertices[offset + 4], vertices[offset + 5]);
        mesh_output.vertices[i].uv = vec2(vertices[offset + 6], vertices[offset + 7]);
    };

    // Set primitive data

    // Define a color based on the position in the total set of workgroups
    let colorMask = vec4<f32>(
        vec3f(mesh_input.global_invocation_id.xyz) / vec3f(mesh_input.num_workgroups.xyz),
        1.0
    );

    for (var i: i32 = 0; i < 12; i++) {
        let offset = i * 3;
        // Set the indices for this primitive
        mesh_output.primitives[i].indices = vec3<u32>(u32(indices[offset]), u32(indices[offset + 1]), u32(indices[offset + 2]));
        // cull primitives based on the current time and offset by their position
        mesh_output.primitives[i].cull = sin(globals.time * 10. + f32(mesh_input.global_invocation_id.x) + f32(mesh_input.global_invocation_id.y) + f32(mesh_input.global_invocation_id.z) ) > 0.8;
        // Apply a color to this primitive, multiplied by the task payload's color
        mesh_output.primitives[i].colorMask = colorMask * taskPayload.colorMask;
    };
}