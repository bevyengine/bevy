use bevy_core_pipeline::prepass::ViewPrepassTextures;
use bevy_render::render_resource::{
    binding_types::{
        texture_2d, texture_2d_multisampled, texture_2d_u32, texture_depth_2d,
        texture_depth_2d_multisampled,
    },
    BindGroupLayoutEntryBuilder, TextureAspect, TextureSampleType, TextureView,
    TextureViewDescriptor,
};
use bevy_utils::default;
use smallvec::SmallVec;

use crate::MeshPipelineViewLayoutKey;

pub fn get_bind_group_layout_entries(
    layout_key: MeshPipelineViewLayoutKey,
) -> SmallVec<[BindGroupLayoutEntryBuilder; 4]> {
    let mut result = SmallVec::<[BindGroupLayoutEntryBuilder; 4]>::new();

    let multisampled = layout_key.contains(MeshPipelineViewLayoutKey::MULTISAMPLED);

    if layout_key.contains(MeshPipelineViewLayoutKey::DEPTH_PREPASS) {
        result.push(
            // Depth texture
            if multisampled {
                texture_depth_2d_multisampled()
            } else {
                texture_depth_2d()
            },
        );
    }

    if layout_key.contains(MeshPipelineViewLayoutKey::NORMAL_PREPASS) {
        result.push(
            // Normal texture
            if multisampled {
                texture_2d_multisampled(TextureSampleType::Float { filterable: false })
            } else {
                texture_2d(TextureSampleType::Float { filterable: false })
            },
        );
    }

    if layout_key.contains(MeshPipelineViewLayoutKey::MOTION_VECTOR_PREPASS) {
        result.push(
            // Motion Vectors texture
            if multisampled {
                texture_2d_multisampled(TextureSampleType::Float { filterable: false })
            } else {
                texture_2d(TextureSampleType::Float { filterable: false })
            },
        );
    }

    if layout_key.contains(MeshPipelineViewLayoutKey::DEFERRED_PREPASS) {
        result.push(
            // Deferred texture
            texture_2d_u32(),
        );
    }

    result
}

pub fn get_bindings(prepass_textures: Option<&ViewPrepassTextures>) -> [Option<TextureView>; 4] {
    let depth_desc = TextureViewDescriptor {
        label: Some("prepass_depth"),
        aspect: TextureAspect::DepthOnly,
        ..default()
    };
    let depth_view = prepass_textures
        .and_then(|x| x.depth.as_ref())
        .map(|texture| texture.texture.create_view(&depth_desc));

    let normal_view = prepass_textures
        .and_then(|x| x.normal.as_ref())
        .map(|texture| texture.default_view.clone());

    let motion_vectors_view = prepass_textures
        .and_then(|x| x.motion_vectors.as_ref())
        .map(|texture| texture.default_view.clone());

    let deferred_view = prepass_textures
        .and_then(|x| x.deferred.as_ref())
        .map(|texture| texture.default_view.clone());

    [depth_view, normal_view, motion_vectors_view, deferred_view]
}
