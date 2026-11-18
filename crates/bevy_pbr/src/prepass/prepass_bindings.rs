use bevy_core_pipeline::prepass::ViewPrepassTextures;
use bevy_render::render_resource::{
    binding_types::{
        texture_2d, texture_2d_multisampled, texture_depth_2d, texture_depth_2d_multisampled,
    },
    BindGroupLayoutEntryBuilder, TextureSampleType, TextureView,
};

use crate::MeshPipelineViewLayoutKey;

/// Returns the layout entries for the prepass bindings of the mesh view bind group.
///
/// `deferred_specular_tint` is whether the view has a deferred specular tint texture.
pub fn get_bind_group_layout_entries(
    layout_key: MeshPipelineViewLayoutKey,
    deferred_specular_tint: bool,
) -> [Option<BindGroupLayoutEntryBuilder>; 5] {
    let mut entries: [Option<BindGroupLayoutEntryBuilder>; 5] = [None; 5];

    let multisampled = layout_key.contains(MeshPipelineViewLayoutKey::MULTISAMPLED);

    if layout_key.contains(MeshPipelineViewLayoutKey::DEPTH_PREPASS) {
        // Depth texture
        entries[0] = if multisampled {
            Some(texture_depth_2d_multisampled())
        } else {
            Some(texture_depth_2d())
        };
    }

    if layout_key.contains(MeshPipelineViewLayoutKey::NORMAL_PREPASS) {
        // Normal texture
        entries[1] = if multisampled {
            Some(texture_2d_multisampled(TextureSampleType::Float {
                filterable: false,
            }))
        } else {
            Some(texture_2d(TextureSampleType::Float { filterable: false }))
        };
    }

    if layout_key.contains(MeshPipelineViewLayoutKey::MOTION_VECTOR_PREPASS) {
        // Motion Vectors texture
        entries[2] = if multisampled {
            Some(texture_2d_multisampled(TextureSampleType::Float {
                filterable: false,
            }))
        } else {
            Some(texture_2d(TextureSampleType::Float { filterable: false }))
        };
    }

    if layout_key.contains(MeshPipelineViewLayoutKey::DEFERRED_PREPASS) {
        // Deferred texture
        entries[3] = Some(texture_2d(TextureSampleType::Uint));
        if deferred_specular_tint {
            // Deferred specular tint texture
            entries[4] = Some(texture_2d(TextureSampleType::Uint));
        }
    }

    entries
}

pub fn get_bindings(prepass_textures: Option<&ViewPrepassTextures>) -> [Option<TextureView>; 5] {
    [
        prepass_textures.and_then(|pt| pt.depth_only_view().cloned()),
        prepass_textures.and_then(|pt| pt.normal_view().cloned()),
        prepass_textures.and_then(|pt| pt.motion_vectors_view().cloned()),
        prepass_textures.and_then(|pt| pt.deferred_view().cloned()),
        prepass_textures.and_then(|pt| pt.deferred_specular_tint_view().cloned()),
    ]
}
