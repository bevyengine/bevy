use bevy_camera::visibility::RenderLayers;

/// Packs render layers into a bit mask.
pub(crate) fn render_layers_to_mask(
    render_layers: Option<&RenderLayers>,
    mask_bits: u32,
    mask: u32,
) -> u32 {
    let bits = render_layers.unwrap_or_default().bits();
    let low_bits = bits.first().copied().unwrap_or_default();
    let unsupported_bits =
        (low_bits >> mask_bits) != 0 || bits.iter().skip(1).any(|&extra_bits| extra_bits != 0);

    if unsupported_bits {
        mask
    } else {
        (low_bits as u32) & mask
    }
}
