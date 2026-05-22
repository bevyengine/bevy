use bevy_core_pipeline::prepass::ViewPrepassTextures;
use bevy_render::render_resource::{
    binding_types::{
        texture_2d, texture_2d_array, texture_2d_multisampled, texture_depth_2d,
        texture_depth_2d_multisampled,
    },
    BindGroupLayoutEntryBuilder, TextureAspect, TextureSampleType, TextureView,
    TextureViewDescriptor, TextureViewDimension,
};
use bevy_utils::default;

use crate::MeshPipelineViewLayoutKey;

pub fn get_bind_group_layout_entries(
    layout_key: MeshPipelineViewLayoutKey,
) -> [Option<BindGroupLayoutEntryBuilder>; 4] {
    let mut entries: [Option<BindGroupLayoutEntryBuilder>; 4] = [None; 4];

    let multisampled = layout_key.contains(MeshPipelineViewLayoutKey::MULTISAMPLED);
    // WGSL has no multisampled-array texture type, so the MSAA + multiview
    // combination keeps the single-layer multisampled shape. Mirrors the
    // shader-side `#ifdef MULTISAMPLED` / `#ifdef MULTIVIEW` interleave in
    // `mesh_view_bindings.wgsl`.
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
/// typed WGSL bindings. The underlying textures are still single-layer this
/// session; per-eye layers will come with L7b-write.
pub fn get_bindings(
    prepass_textures: Option<&ViewPrepassTextures>,
    multiview_array: bool,
    deferred_multiview: bool,
) -> [Option<TextureView>; 4] {
    let view_dimension = if multiview_array {
        Some(TextureViewDimension::D2Array)
    } else {
        None
    };

    let depth_desc = TextureViewDescriptor {
        label: Some("prepass_depth"),
        aspect: TextureAspect::DepthOnly,
        dimension: view_dimension,
        ..default()
    };
    let depth_view = prepass_textures
        .and_then(|x| x.depth.as_ref())
        .map(|texture| texture.texture.texture.create_view(&depth_desc));

    let make_array_view = |label: &'static str, cached: &bevy_render::texture::CachedTexture| {
        cached.texture.create_view(&TextureViewDescriptor {
            label: Some(label),
            dimension: Some(TextureViewDimension::D2Array),
            ..default()
        })
    };

    let normal_view = prepass_textures.and_then(|pt| {
        pt.normal.as_ref().map(|att| {
            if multiview_array {
                make_array_view("prepass_normal_array", &att.texture)
            } else {
                att.texture.default_view.clone()
            }
        })
    });
    let motion_view = prepass_textures.and_then(|pt| {
        pt.motion_vectors.as_ref().map(|att| {
            if multiview_array {
                make_array_view("prepass_motion_vectors_array", &att.texture)
            } else {
                att.texture.default_view.clone()
            }
        })
    });
    let deferred_view = prepass_textures.and_then(|pt| {
        pt.deferred.as_ref().map(|att| {
            if deferred_multiview {
                make_array_view("prepass_deferred_array", &att.texture)
            } else {
                att.texture.default_view.clone()
            }
        })
    });

    [depth_view, normal_view, motion_view, deferred_view]
}
