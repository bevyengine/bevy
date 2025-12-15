use bevy_asset::Handle;
use bevy_image::Image;

use gltf::Material;

use serde_json::Value;

#[cfg(feature = "pbr_anisotropy_texture")]
use {crate::loader::gltf_ext::material::parse_material_extension_texture, bevy_pbr::UvChannel};

/// Parsed data from the `KHR_materials_anisotropy` extension.
///
/// See the specification:
/// <https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_anisotropy/README.md>
#[derive(Default)]
pub(crate) struct AnisotropyExtension {
    pub(crate) anisotropy_strength: Option<f64>,
    pub(crate) anisotropy_rotation: Option<f64>,
    #[cfg(feature = "pbr_anisotropy_texture")]
    pub(crate) anisotropy_channel: UvChannel,
    #[cfg(feature = "pbr_anisotropy_texture")]
    pub(crate) anisotropy_texture: Option<Handle<Image>>,
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
    pub(crate) fn parse(
        material: &Material,
        textures: &[Handle<Image>],
    ) -> Option<AnisotropyExtension> {
        let extension = material
            .extensions()?
            .get("KHR_materials_anisotropy")?
            .as_object()?;

        #[cfg(feature = "pbr_anisotropy_texture")]
        let (anisotropy_channel, anisotropy_texture) = parse_material_extension_texture(
            material,
            extension,
            "anisotropyTexture",
            "anisotropy",
            textures,
        );

        Some(AnisotropyExtension {
            anisotropy_strength: extension.get("anisotropyStrength").and_then(Value::as_f64),
            anisotropy_rotation: extension.get("anisotropyRotation").and_then(Value::as_f64),
            #[cfg(feature = "pbr_anisotropy_texture")]
            anisotropy_channel,
            #[cfg(feature = "pbr_anisotropy_texture")]
            anisotropy_texture,
        })
    }
}
