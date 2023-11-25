use bevy_core_pipeline::prepass::ViewPrepassTextures;
use bevy_render::render_resource::{
    BindGroupLayoutEntry, BindingType, ShaderStages, TextureAspect, TextureSampleType, TextureView,
    TextureViewDescriptor, TextureViewDimension,
};
use bevy_utils::default;
use smallvec::SmallVec;

use crate::MeshPipelineViewLayoutKey;

pub fn get_bind_group_layout_entries(
    bindings: [u32; 4],
    layout_key: MeshPipelineViewLayoutKey,
) -> SmallVec<[BindGroupLayoutEntry; 4]> {
    let mut result = SmallVec::<[BindGroupLayoutEntry; 4]>::new();

    let multisampled = layout_key.contains(MeshPipelineViewLayoutKey::MULTISAMPLED);

    if layout_key.contains(MeshPipelineViewLayoutKey::DEPTH_PREPASS) {
        result.push(
            // Depth texture
            BindGroupLayoutEntry {
                binding: bindings[0],
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    multisampled,
                    sample_type: TextureSampleType::Depth,
                    view_dimension: TextureViewDimension::D2,
                },
                count: None,
            },
        );
    }

    if layout_key.contains(MeshPipelineViewLayoutKey::NORMAL_PREPASS) {
        result.push(
            // Normal texture
            BindGroupLayoutEntry {
                binding: bindings[1],
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    multisampled,
                    sample_type: TextureSampleType::Float { filterable: false },
                    view_dimension: TextureViewDimension::D2,
                },
                count: None,
            },
        );
    }

    if layout_key.contains(MeshPipelineViewLayoutKey::MOTION_VECTOR_PREPASS) {
        result.push(
            // Motion Vectors texture
            BindGroupLayoutEntry {
                binding: bindings[2],
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    multisampled,
                    sample_type: TextureSampleType::Float { filterable: false },
                    view_dimension: TextureViewDimension::D2,
                },
                count: None,
            },
        );
    }

    if layout_key.contains(MeshPipelineViewLayoutKey::DEFERRED_PREPASS) {
        result.push(
            // Deferred texture
            BindGroupLayoutEntry {
                binding: bindings[3],
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    multisampled: false,
                    sample_type: TextureSampleType::Uint,
                    view_dimension: TextureViewDimension::D2,
                },
                count: None,
            },
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
