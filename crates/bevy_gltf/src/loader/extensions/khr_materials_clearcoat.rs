use bevy_asset::Handle;
use bevy_image::Image;

use gltf::Material;

use serde_json::Value;

#[cfg(feature = "pbr_multi_layer_material_textures")]
use {crate::loader::gltf_ext::material::parse_material_extension_texture, bevy_pbr::UvChannel};

/// Parsed data from the `KHR_materials_clearcoat` extension.
///
/// See the specification:
/// <https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_clearcoat/README.md>
#[derive(Default)]
pub(crate) struct ClearcoatExtension {
    pub(crate) clearcoat_factor: Option<f64>,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub(crate) clearcoat_channel: UvChannel,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub(crate) clearcoat_texture: Option<Handle<Image>>,
    pub(crate) clearcoat_roughness_factor: Option<f64>,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub(crate) clearcoat_roughness_channel: UvChannel,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub(crate) clearcoat_roughness_texture: Option<Handle<Image>>,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub(crate) clearcoat_normal_channel: UvChannel,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub(crate) clearcoat_normal_texture: Option<Handle<Image>>,
}

impl ClearcoatExtension {
    #[expect(
        clippy::allow_attributes,
        reason = "`unused_variables` is not always linted"
    )]
    #[allow(
        unused_variables,
        reason = "Depending on what features are used to compile this crate, certain parameters may end up unused."
    )]
    pub(crate) fn parse(
        material: &Material,
        textures: &[Handle<Image>],
    ) -> Option<ClearcoatExtension> {
        let extension = material
            .extensions()?
            .get("KHR_materials_clearcoat")?
            .as_object()?;

        #[cfg(feature = "pbr_multi_layer_material_textures")]
        let (clearcoat_channel, clearcoat_texture) = parse_material_extension_texture(
            material,
            extension,
            "clearcoatTexture",
            "clearcoat",
            textures,
        );

        #[cfg(feature = "pbr_multi_layer_material_textures")]
        let (clearcoat_roughness_channel, clearcoat_roughness_texture) =
            parse_material_extension_texture(
                material,
                extension,
                "clearcoatRoughnessTexture",
                "clearcoat roughness",
                textures,
            );

        #[cfg(feature = "pbr_multi_layer_material_textures")]
        let (clearcoat_normal_channel, clearcoat_normal_texture) = parse_material_extension_texture(
            material,
            extension,
            "clearcoatNormalTexture",
            "clearcoat normal",
            textures,
        );

        Some(ClearcoatExtension {
            clearcoat_factor: extension.get("clearcoatFactor").and_then(Value::as_f64),
            clearcoat_roughness_factor: extension
                .get("clearcoatRoughnessFactor")
                .and_then(Value::as_f64),
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_channel,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_texture,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_roughness_channel,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_roughness_texture,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_normal_channel,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_normal_texture,
        })
    }
}
