use bevy_math::Affine2;
use bevy_pbr::UvChannel;
use bevy_render::alpha::AlphaMode;

use gltf::{json::texture::Info, Material};

use serde_json::value;

use crate::GltfAssetLabel;

use super::texture::texture_transform_to_affine2;

#[cfg(any(
    feature = "pbr_specular_textures",
    feature = "pbr_multi_layer_material_textures"
))]
use {
    super::texture::texture_handle_from_info,
    bevy_asset::{Handle, LoadContext},
    bevy_image::Image,
    gltf::Document,
    serde_json::{Map, Value},
};

/// Parses a texture that's part of a material extension block and returns its
/// UV channel and image reference.
#[cfg(any(
    feature = "pbr_specular_textures",
    feature = "pbr_multi_layer_material_textures"
))]
pub(crate) fn parse_material_extension_texture(
    material: &Material,
    load_context: &mut LoadContext,
    document: &Document,
    extension: &Map<String, Value>,
    texture_name: &str,
    texture_kind: &str,
) -> (UvChannel, Option<Handle<Image>>) {
    match extension
        .get(texture_name)
        .and_then(|value| value::from_value::<Info>(value.clone()).ok())
    {
        Some(json_info) => (
            uv_channel(material, texture_kind, json_info.tex_coord),
            Some(texture_handle_from_info(&json_info, document, load_context)),
        ),
        None => (UvChannel::default(), None),
    }
}

pub(crate) fn uv_channel(material: &Material, texture_kind: &str, tex_coord: u32) -> UvChannel {
    match tex_coord {
        0 => UvChannel::Uv0,
        1 => UvChannel::Uv1,
        _ => {
            let material_name = material
                .name()
                .map(|n| format!("the material \"{n}\""))
                .unwrap_or_else(|| "an unnamed material".to_string());
            let material_index = material
                .index()
                .map(|i| format!("index {i}"))
                .unwrap_or_else(|| "default".to_string());
            tracing::warn!(
                    "Only 2 UV Channels are supported, but {material_name} ({material_index}) \
                    has the TEXCOORD attribute {} on texture kind {texture_kind}, which will fallback to 0.",
                    tex_coord,
                );
            UvChannel::Uv0
        }
    }
}

pub(crate) fn alpha_mode(material: &Material) -> AlphaMode {
    match material.alpha_mode() {
        gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
        gltf::material::AlphaMode::Mask => AlphaMode::Mask(material.alpha_cutoff().unwrap_or(0.5)),
        gltf::material::AlphaMode::Blend => AlphaMode::Blend,
    }
}

/// Returns the index (within the `textures` array) of the texture with the
/// given field name in the data for the material extension with the given name,
/// if there is one.
pub(crate) fn extension_texture_index(
    material: &Material,
    extension_name: &str,
    texture_field_name: &str,
) -> Option<usize> {
    Some(
        value::from_value::<Info>(
            material
                .extensions()?
                .get(extension_name)?
                .as_object()?
                .get(texture_field_name)?
                .clone(),
        )
        .ok()?
        .index
        .value(),
    )
}

/// Returns true if the material needs mesh tangents in order to be successfully
/// rendered.
///
/// We generate them if this function returns true.
pub(crate) fn needs_tangents(material: &Material) -> bool {
    [
        material.normal_texture().is_some(),
        #[cfg(feature = "pbr_multi_layer_material_textures")]
        extension_texture_index(
            material,
            "KHR_materials_clearcoat",
            "clearcoatNormalTexture",
        )
        .is_some(),
    ]
    .into_iter()
    .reduce(|a, b| a || b)
    .unwrap_or(false)
}

pub(crate) fn warn_on_differing_texture_transforms(
    material: &Material,
    info: &gltf::texture::Info,
    texture_transform: Affine2,
    texture_kind: &str,
) {
    let has_differing_texture_transform = info
        .texture_transform()
        .map(texture_transform_to_affine2)
        .is_some_and(|t| t != texture_transform);
    if has_differing_texture_transform {
        let material_name = material
            .name()
            .map(|n| format!("the material \"{n}\""))
            .unwrap_or_else(|| "an unnamed material".to_string());
        let texture_name = info
            .texture()
            .name()
            .map(|n| format!("its {texture_kind} texture \"{n}\""))
            .unwrap_or_else(|| format!("its unnamed {texture_kind} texture"));
        let material_index = material
            .index()
            .map(|i| format!("index {i}"))
            .unwrap_or_else(|| "default".to_string());
        tracing::warn!(
            "Only texture transforms on base color textures are supported, but {material_name} ({material_index}) \
            has a texture transform on {texture_name} (index {}), which will be ignored.", info.texture().index()
        );
    }
}

pub(crate) fn material_label(material: &Material, is_scale_inverted: bool) -> GltfAssetLabel {
    if let Some(index) = material.index() {
        GltfAssetLabel::Material {
            index,
            is_scale_inverted,
        }
    } else {
        GltfAssetLabel::DefaultMaterial
    }
}
