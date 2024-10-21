use serde_json::Value;

use gltf::{Document, Material};

use bevy_asset::LoadContext;

#[cfg(feature = "pbr_multi_layer_material_textures")]
use {bevy_asset::Handle, bevy_image::Image, bevy_pbr::UvChannel, gltf::json, serde_json::value};

/// Parsed data from the `KHR_materials_clearcoat` extension.
///
/// See the specification:
/// <https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_clearcoat/README.md>
#[derive(Default)]
pub struct ClearcoatExtension {
    pub clearcoat_factor: Option<f64>,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_channel: UvChannel,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_texture: Option<Handle<Image>>,
    pub clearcoat_roughness_factor: Option<f64>,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_roughness_channel: UvChannel,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_roughness_texture: Option<Handle<Image>>,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_normal_channel: UvChannel,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_normal_texture: Option<Handle<Image>>,
}

impl ClearcoatExtension {
    #[allow(unused_variables)]
    pub fn parse(
        load_context: &mut LoadContext,
        document: &Document,
        material: &Material,
    ) -> Option<ClearcoatExtension> {
        let extension = material
            .extensions()?
            .get("KHR_materials_clearcoat")?
            .as_object()?;

        #[cfg(feature = "pbr_multi_layer_material_textures")]
        let (clearcoat_channel, clearcoat_texture) = extension
            .get("clearcoatTexture")
            .and_then(|value| value::from_value::<json::texture::Info>(value.clone()).ok())
            .map(|json_info| {
                (
                    super::GltfMaterial::get_uv_channel(material, "clearcoat", json_info.tex_coord),
                    super::GltfMaterial::texture_handle_from_info(
                        load_context,
                        document,
                        &json_info,
                    ),
                )
            })
            .unzip();

        #[cfg(feature = "pbr_multi_layer_material_textures")]
        let (clearcoat_roughness_channel, clearcoat_roughness_texture) = extension
            .get("clearcoatRoughnessTexture")
            .and_then(|value| value::from_value::<json::texture::Info>(value.clone()).ok())
            .map(|json_info| {
                (
                    super::GltfMaterial::get_uv_channel(
                        material,
                        "clearcoat roughness",
                        json_info.tex_coord,
                    ),
                    super::GltfMaterial::texture_handle_from_info(
                        load_context,
                        document,
                        &json_info,
                    ),
                )
            })
            .unzip();

        #[cfg(feature = "pbr_multi_layer_material_textures")]
        let (clearcoat_normal_channel, clearcoat_normal_texture) = extension
            .get("clearcoatNormalTexture")
            .and_then(|value| value::from_value::<json::texture::Info>(value.clone()).ok())
            .map(|json_info| {
                (
                    super::GltfMaterial::get_uv_channel(
                        material,
                        "clearcoat normal",
                        json_info.tex_coord,
                    ),
                    super::GltfMaterial::texture_handle_from_info(
                        load_context,
                        document,
                        &json_info,
                    ),
                )
            })
            .unzip();

        Some(ClearcoatExtension {
            clearcoat_factor: extension.get("clearcoatFactor").and_then(Value::as_f64),
            clearcoat_roughness_factor: extension
                .get("clearcoatRoughnessFactor")
                .and_then(Value::as_f64),
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_channel: clearcoat_channel.unwrap_or_default(),
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_texture,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_roughness_channel: clearcoat_roughness_channel.unwrap_or_default(),
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_roughness_texture,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_normal_channel: clearcoat_normal_channel.unwrap_or_default(),
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_normal_texture,
        })
    }
}
