use bevy_asset::LoadContext;

use gltf::{Document, Material};

use serde_json::Value;

#[cfg(feature = "pbr_specular_textures")]
use {
    crate::loader::gltf_ext::material::parse_material_extension_texture, bevy_asset::Handle,
    bevy_image::Image, bevy_pbr::UvChannel,
};

/// Parsed data from the `KHR_materials_specular` extension.
///
/// We currently don't parse `specularFactor` and `specularTexture`, since
/// they're incompatible with Filament.
///
/// Note that the map is a *specular map*, not a *reflectance map*. In Bevy and
/// Filament terms, the reflectance values in the specular map range from [0.0,
/// 0.5], rather than [0.0, 1.0]. This is an unfortunate
/// `KHR_materials_specular` specification requirement that stems from the fact
/// that glTF is specified in terms of a specular strength model, not the
/// reflectance model that Filament and Bevy use. A workaround, which is noted
/// in the [`StandardMaterial`](bevy_pbr::StandardMaterial) documentation, is to set the reflectance value
/// to 2.0, which spreads the specular map range from [0.0, 1.0] as normal.
///
/// See the specification:
/// <https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_specular/README.md>
#[derive(Default)]
pub(crate) struct SpecularExtension {
    pub(crate) specular_factor: Option<f64>,
    #[cfg(feature = "pbr_specular_textures")]
    pub(crate) specular_channel: UvChannel,
    #[cfg(feature = "pbr_specular_textures")]
    pub(crate) specular_texture: Option<Handle<Image>>,
    pub(crate) specular_color_factor: Option<[f64; 3]>,
    #[cfg(feature = "pbr_specular_textures")]
    pub(crate) specular_color_channel: UvChannel,
    #[cfg(feature = "pbr_specular_textures")]
    pub(crate) specular_color_texture: Option<Handle<Image>>,
}

impl SpecularExtension {
    pub(crate) fn parse(
        _load_context: &mut LoadContext,
        _document: &Document,
        material: &Material,
    ) -> Option<Self> {
        let extension = material
            .extensions()?
            .get("KHR_materials_specular")?
            .as_object()?;

        #[cfg(feature = "pbr_specular_textures")]
        let (_specular_channel, _specular_texture) = parse_material_extension_texture(
            material,
            _load_context,
            _document,
            extension,
            "specularTexture",
            "specular",
        );

        #[cfg(feature = "pbr_specular_textures")]
        let (_specular_color_channel, _specular_color_texture) = parse_material_extension_texture(
            material,
            _load_context,
            _document,
            extension,
            "specularColorTexture",
            "specular color",
        );

        Some(SpecularExtension {
            specular_factor: extension.get("specularFactor").and_then(Value::as_f64),
            #[cfg(feature = "pbr_specular_textures")]
            specular_channel: _specular_channel,
            #[cfg(feature = "pbr_specular_textures")]
            specular_texture: _specular_texture,
            specular_color_factor: extension
                .get("specularColorFactor")
                .and_then(Value::as_array)
                .and_then(|json_array| {
                    if json_array.len() < 3 {
                        None
                    } else {
                        Some([
                            json_array[0].as_f64()?,
                            json_array[1].as_f64()?,
                            json_array[2].as_f64()?,
                        ])
                    }
                }),
            #[cfg(feature = "pbr_specular_textures")]
            specular_color_channel: _specular_color_channel,
            #[cfg(feature = "pbr_specular_textures")]
            specular_color_texture: _specular_color_texture,
        })
    }
}
