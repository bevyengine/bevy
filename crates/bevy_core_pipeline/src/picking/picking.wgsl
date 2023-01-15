#define_import_path bevy_core_pipeline::picking

// If e.g. a sprite has slightly transparent pixels, we make that opaque (by setting alpha to 1.0)
// for picking purposes.
// If we don't do this, blending will occur on the entity index value, which makes no sense.
//
// An alternative is to truncate the alpha to 0.0 unless it's 1.0, but that would shrink
// which parts of the translucent entity are pickable. We just have to make a choice,
// and here we expand the pickable area instead of shrinking it.
fn picking_alpha(a: f32) -> f32 {
    return ceil(a);
}

// In short, this is to pack a u32 into a vec3<f32>.
// See bevy_render/src/picking/mod.rs for further explanation.
fn entity_index_to_vec3_f32(entity_index: u32) -> vec3<f32> {
    // Such that the CPU side can know if there is an entity at a given pixel.
    // The CPU-side entity index 0 is represented as index 1 in the texture.
    // CPU-side can then identify index 0 in the texture as "no entity",
    // and index 1 as entity 0.
    let virtual_entity_index = entity_index + 1u;

    let mask_8 = 0x000000FFu;
    let mask_12 = 0x00000FFFu;

    let lower_8 = virtual_entity_index & mask_8;
    let mid_12 = (virtual_entity_index >> 8u) & mask_12;
    let up_12 = (virtual_entity_index >> 20u) & mask_12;

    return vec3(
        f32(lower_8),
        f32(mid_12),
        f32(up_12),
    );
}
