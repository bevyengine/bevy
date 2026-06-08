use bevy_asset::{AssetPath, Handle};
use bevy_image::Image;

use gltf::Material;

use serde_json::Value;

#[cfg(feature = "pbr_specular_textures")]
use {crate::loader::gltf_ext::material::parse_material_extension_texture, bevy_mesh::UvChannel};

/// Parsed data from the `KHR_materials_specular` extension.
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
        asset_path: AssetPath<'_>,
    ) -> Option<Self> {
        let extension = material
            .extensions()?
            .get("KHR_materials_specular")?
            .as_object()?;

        #[cfg(feature = "pbr_specular_textures")]
        let (_specular_channel, _specular_texture) = parse_material_extension_texture(
            material,
            extension,
            "specularTexture",
            "specular",
            textures,
            asset_path.clone(),
        );

        #[cfg(feature = "pbr_specular_textures")]
        let (_specular_color_channel, _specular_color_texture) = parse_material_extension_texture(
            material,
            extension,
            "specularColorTexture",
            "specular color",
            textures,
            asset_path,
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
