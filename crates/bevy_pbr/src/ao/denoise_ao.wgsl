// 5x5 bilaterial filter (edge-preserving blur)
// https://people.csail.mit.edu/sparis/bf_course/course_notes.pdf

@compute
@workgroup_size(8, 8, 1)
fn denoise(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel_coordinates = global_id.xy * vec2<u32>(2u, 1u);
}
