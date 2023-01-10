#define_import_path bevy_core_pipeline::picking

// TODO: Describe why/what
fn entity_index_to_vec3_f32(entity_index: u32) -> vec3<f32> {
    let mask_8 = 0x000000FFu;
    let mask_12 = 0x00000FFFu;

    let lower_8 = entity_index & mask_8;
    let mid_12 = (entity_index >> 8u) & mask_12;
    let up_12 = (entity_index >> 20u) & mask_12;

    return vec3(
        f32(lower_8),
        f32(mid_12),
        f32(up_12),
    );
}
