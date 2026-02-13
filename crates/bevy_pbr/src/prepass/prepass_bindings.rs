use bevy_core_pipeline::prepass::ViewPrepassTextures;
use bevy_render::render_resource::{
    binding_types::texture_2d, BindGroupLayoutEntryBuilder, TextureSampleType, TextureView,
};

use crate::MeshPipelineViewLayoutKey;

/// Get the bind group entries of prepass textures.
///
/// If `float32_filterable` [`bevy_render::settings::WgpuFeatures::FLOAT32_FILTERABLE`] is true,
/// depth texture binding will be filterable.
pub fn get_bind_group_layout_entries(
    layout_key: MeshPipelineViewLayoutKey,
    float32_filterable: bool,
) -> [Option<BindGroupLayoutEntryBuilder>; 4] {
    let mut entries: [Option<BindGroupLayoutEntryBuilder>; 4] = [None; 4];

    if layout_key.contains(MeshPipelineViewLayoutKey::DEPTH_PREPASS) {
        // Depth texture
        entries[0] = Some(texture_2d(TextureSampleType::Float {
            filterable: float32_filterable,
        }));
    }

    if layout_key.contains(MeshPipelineViewLayoutKey::NORMAL_PREPASS) {
        // Normal texture
        entries[1] = Some(texture_2d(TextureSampleType::Float { filterable: true }));
    }

    if layout_key.contains(MeshPipelineViewLayoutKey::MOTION_VECTOR_PREPASS) {
        // Motion Vectors texture
        entries[2] = Some(texture_2d(TextureSampleType::Float { filterable: true }));
    }

    if layout_key.contains(MeshPipelineViewLayoutKey::DEFERRED_PREPASS) {
        // Deferred texture
        entries[3] = Some(texture_2d(TextureSampleType::Uint));
    }

    entries
}

pub fn get_bindings(prepass_textures: Option<&ViewPrepassTextures>) -> [Option<TextureView>; 4] {
    [
        prepass_textures.and_then(|pt| pt.depth_view().cloned()),
        prepass_textures.and_then(|pt| pt.normal_view().cloned()),
        prepass_textures.and_then(|pt| pt.motion_vectors_view().cloned()),
        prepass_textures.and_then(|pt| pt.deferred_view().cloned()),
    ]
}
