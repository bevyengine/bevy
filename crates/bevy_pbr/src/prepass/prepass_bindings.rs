use bevy_core_pipeline::prepass::ViewPrepassTextures;
use bevy_render::render_resource::{
    binding_types::{
        texture_2d, texture_2d_array, texture_2d_multisampled, texture_depth_2d,
        texture_depth_2d_multisampled,
    },
    BindGroupLayoutEntryBuilder, TextureAspect, TextureSampleType, TextureView,
    TextureViewDescriptor, TextureViewDimension,
};

use crate::MeshPipelineViewLayoutKey;

pub fn get_bind_group_layout_entries(
    layout_key: MeshPipelineViewLayoutKey,
) -> [Option<BindGroupLayoutEntryBuilder>; 4] {
    let mut entries: [Option<BindGroupLayoutEntryBuilder>; 4] = [None; 4];

    let multisampled = layout_key.contains(MeshPipelineViewLayoutKey::MULTISAMPLED);
    // WGSL has no multisampled-array texture type, so the MSAA + multiview
    // combination keeps the single-layer multisampled shape. Mirrors the
    // shader-side `@if(MULTISAMPLED)` / `@if(MULTIVIEW)` conditions in
    // `mesh_view_bindings.wesl`.
    let multiview_array =
        !multisampled && layout_key.contains(MeshPipelineViewLayoutKey::MULTIVIEW);

    if layout_key.contains(MeshPipelineViewLayoutKey::DEPTH_PREPASS) {
        // Depth texture
        entries[0] = if multisampled {
            Some(texture_depth_2d_multisampled())
        } else if multiview_array {
            Some(texture_2d_array(TextureSampleType::Depth))
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
        } else if multiview_array {
            Some(texture_2d_array(TextureSampleType::Float {
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
        } else if multiview_array {
            Some(texture_2d_array(TextureSampleType::Float {
                filterable: false,
            }))
        } else {
            Some(texture_2d(TextureSampleType::Float { filterable: false }))
        };
    }

    if layout_key.contains(MeshPipelineViewLayoutKey::DEFERRED_PREPASS) {
        // Deferred texture (never multisampled)
        entries[3] = if layout_key.contains(MeshPipelineViewLayoutKey::MULTIVIEW) {
            Some(texture_2d_array(TextureSampleType::Uint))
        } else {
            Some(texture_2d(TextureSampleType::Uint))
        };
    }

    entries
}

/// Returns texture views for the four prepass texture slots, picking
/// `D2Array` views under `multiview_array` so they line up with the array-
/// typed WGSL bindings. Under multiview each texture carries `view_count`
/// layers; the `D2Array` view wraps the full array and the consumer reads
/// its eye's slice via `current_view_index`.
///
/// The non-multiview path returns the prepass textures' prebuilt views
/// unchanged, so single-view rendering is unaffected.
pub fn get_bindings(
    prepass_textures: Option<&ViewPrepassTextures>,
    multiview_array: bool,
    deferred_multiview: bool,
) -> [Option<TextureView>; 4] {
    let make_array_view = |label: &'static str, cached: &bevy_render::texture::CachedTexture| {
        cached.texture.create_view(&TextureViewDescriptor {
            label: Some(label),
            dimension: Some(TextureViewDimension::D2Array),
            ..Default::default()
        })
    };

    let depth_view = if multiview_array {
        prepass_textures
            .and_then(|x| x.depth.as_ref())
            .map(|attachment| {
                attachment
                    .texture
                    .texture
                    .create_view(&TextureViewDescriptor {
                        label: Some("prepass_depth_array"),
                        aspect: TextureAspect::DepthOnly,
                        dimension: Some(TextureViewDimension::D2Array),
                        ..Default::default()
                    })
            })
    } else {
        prepass_textures.and_then(|pt| pt.depth_only_view().cloned())
    };

    let normal_view = if multiview_array {
        prepass_textures.and_then(|pt| {
            pt.normal
                .as_ref()
                .map(|att| make_array_view("prepass_normal_array", &att.texture))
        })
    } else {
        prepass_textures.and_then(|pt| pt.normal_view().cloned())
    };

    let motion_view = if multiview_array {
        prepass_textures.and_then(|pt| {
            pt.motion_vectors
                .as_ref()
                .map(|att| make_array_view("prepass_motion_vectors_array", &att.texture))
        })
    } else {
        prepass_textures.and_then(|pt| pt.motion_vectors_view().cloned())
    };

    let deferred_view = if deferred_multiview {
        prepass_textures.and_then(|pt| {
            pt.deferred
                .as_ref()
                .map(|att| make_array_view("prepass_deferred_array", &att.texture))
        })
    } else {
        prepass_textures.and_then(|pt| pt.deferred_view().cloned())
    };

    [depth_view, normal_view, motion_view, deferred_view]
}
