import super::{
    bindings::{globals, TaskPayload},
};
enable wgpu_mesh_shader;

var<task_payload> taskPayload: TaskPayload;

@task
@payload(taskPayload)
@workgroup_size(1)
fn task() -> @builtin(mesh_task_size) vec3<u32> {

    // This color is multiplied against the color of each cube.
    // Changing it modifies all cube colors.
    taskPayload.colorMask = vec4(1.0, 1.0, 1.0, 1.0);

    // how many mesh shaders get dispatched grows over time
    // until it passes a gpu limit. When it hits the limit all
    // cubes disappear, showing what can happen if you choose the
    // wrong values.
    return vec3u(u32(globals.time));
}