use bevy_core_pipeline::prepass::ViewPrepassTextures;
use bevy_render::{
    frame_graph::{
        FrameGraph, TransientTexture, GraphResourceNodeHandle, ResourceMaterial, TextureViewInfo,
    },
    render_resource::{
        binding_types::{
            texture_2d, texture_2d_multisampled, texture_depth_2d, texture_depth_2d_multisampled,
        },
        BindGroupLayoutEntryBuilder, TextureAspect, TextureSampleType,
    },
};
use bevy_utils::default;

use crate::MeshPipelineViewLayoutKey;

pub fn get_bind_group_layout_entries(
    layout_key: MeshPipelineViewLayoutKey,
) -> [Option<BindGroupLayoutEntryBuilder>; 4] {
    let mut entries: [Option<BindGroupLayoutEntryBuilder>; 4] = [None; 4];

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
    }

    entries
}

pub fn get_bindings(
    prepass_textures: Option<&ViewPrepassTextures>,
    frame_graph: &mut FrameGraph,
) -> [Option<(GraphResourceNodeHandle<TransientTexture>, TextureViewInfo)>; 4] {
    let depth_desc = TextureViewInfo {
        label: Some("prepass_depth".into()),
        aspect: TextureAspect::DepthOnly,
        ..default()
    };
    let depth_view = prepass_textures
        .and_then(|x| x.depth.as_ref())
        .map(|texture| {
            let texture = texture.texture.imported(frame_graph);
            (texture, depth_desc.clone())
        });

    [
        depth_view,
        prepass_textures.and_then(|pt| pt.normal(frame_graph)),
        prepass_textures.and_then(|pt| pt.motion_vectors(frame_graph)),
        prepass_textures.and_then(|pt| pt.deferred(frame_graph)),
    ]
}
