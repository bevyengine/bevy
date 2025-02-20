use bevy_asset::LoadContext;

use gltf::{Document, Material};

use serde_json::Value;

#[cfg(feature = "pbr_anisotropy_texture")]
use {
    crate::loader::gltf_ext::{material::MaterialExt, texture::InfoExt},
    bevy_asset::Handle,
    bevy_image::Image,
    bevy_pbr::UvChannel,
    gltf::json::texture::Info,
    serde_json::value,
};

/// Parsed data from the `KHR_materials_anisotropy` extension.
///
/// See the specification:
/// <https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_anisotropy/README.md>
#[derive(Default)]
pub struct AnisotropyExtension {
    pub anisotropy_strength: Option<f64>,
    pub anisotropy_rotation: Option<f64>,
    #[cfg(feature = "pbr_anisotropy_texture")]
    pub anisotropy_channel: UvChannel,
    #[cfg(feature = "pbr_anisotropy_texture")]
    pub anisotropy_texture: Option<Handle<Image>>,
}

impl AnisotropyExtension {
    #[expect(
        clippy::allow_attributes,
        reason = "`unused_variables` is not always linted"
    )]
    #[allow(
        unused_variables,
        reason = "Depending on what features are used to compile this crate, certain parameters may end up unused."
    )]
    pub fn parse(
        load_context: &mut LoadContext,
        document: &Document,
        material: &Material,
    ) -> Option<AnisotropyExtension> {
        let extension = material
            .extensions()?
            .get("KHR_materials_anisotropy")?
            .as_object()?;

        #[cfg(feature = "pbr_anisotropy_texture")]
        let (anisotropy_channel, anisotropy_texture) = extension
            .get("anisotropyTexture")
            .and_then(|value| value::from_value::<Info>(value.clone()).ok())
            .map(|json_info| {
                (
                    material.uv_channel("anisotropy", json_info.tex_coord),
                    json_info.texture_handle(document, load_context),
                )
            })
            .unzip();

        Some(AnisotropyExtension {
            anisotropy_strength: extension.get("anisotropyStrength").and_then(Value::as_f64),
            anisotropy_rotation: extension.get("anisotropyRotation").and_then(Value::as_f64),
            #[cfg(feature = "pbr_anisotropy_texture")]
            anisotropy_channel: anisotropy_channel.unwrap_or_default(),
            #[cfg(feature = "pbr_anisotropy_texture")]
            anisotropy_texture,
        })
    }
}
