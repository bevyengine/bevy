use bevy_asset::{AssetPath, Handle};
use bevy_image::Image;

use gltf::Material;

#[cfg(feature = "pbr_specular_textures")]
use {crate::loader::gltf_ext::material::uv_channel, bevy_mesh::UvChannel};

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
/// in the [`StandardMaterial`](https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html) documentation, is to set the reflectance value
/// to 2.0, which spreads the specular map range from [0.0, 1.0] as normal.
///
/// See the specification:
/// <https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_specular/README.md>
pub(crate) struct SpecularExtension {
    pub(crate) specular_factor: f32,
    #[cfg(feature = "pbr_specular_textures")]
    pub(crate) specular_channel: UvChannel,
    #[cfg(feature = "pbr_specular_textures")]
    pub(crate) specular_texture: Option<Handle<Image>>,
    pub(crate) specular_color_factor: [f32; 3],
    #[cfg(feature = "pbr_specular_textures")]
    pub(crate) specular_color_channel: UvChannel,
    #[cfg(feature = "pbr_specular_textures")]
    pub(crate) specular_color_texture: Option<Handle<Image>>,
}

impl Default for SpecularExtension {
    fn default() -> Self {
        Self {
            specular_factor: 1.0,
            #[cfg(feature = "pbr_specular_textures")]
            specular_channel: UvChannel::default(),
            #[cfg(feature = "pbr_specular_textures")]
            specular_texture: None,
            specular_color_factor: [1.0, 1.0, 1.0],
            #[cfg(feature = "pbr_specular_textures")]
            specular_color_channel: UvChannel::default(),
            #[cfg(feature = "pbr_specular_textures")]
            specular_color_texture: None,
        }
    }
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
        let specular = material.specular()?;

        #[cfg(feature = "pbr_specular_textures")]
        let _specular_channel = specular
            .specular_texture()
            .map(|info| uv_channel(material, "specular", info.tex_coord()))
            .unwrap_or_default();
        #[cfg(feature = "pbr_specular_textures")]
        let _specular_texture = specular.specular_texture().map(|info| {
            textures
                .get(info.texture().index())
                .cloned()
                .unwrap_or_default()
        });

        #[cfg(feature = "pbr_specular_textures")]
        let _specular_color_channel = specular
            .specular_color_texture()
            .map(|info| uv_channel(material, "specular color", info.tex_coord()))
            .unwrap_or_default();
        #[cfg(feature = "pbr_specular_textures")]
        let _specular_color_texture = specular.specular_color_texture().map(|info| {
            textures
                .get(info.texture().index())
                .cloned()
                .unwrap_or_default()
        });

        Some(SpecularExtension {
            specular_factor: specular.specular_factor(),
            #[cfg(feature = "pbr_specular_textures")]
            specular_channel: _specular_channel,
            #[cfg(feature = "pbr_specular_textures")]
            specular_texture: _specular_texture,
            specular_color_factor: specular.specular_color_factor(),
            #[cfg(feature = "pbr_specular_textures")]
            specular_color_channel: _specular_color_channel,
            #[cfg(feature = "pbr_specular_textures")]
            specular_color_texture: _specular_color_texture,
        })
    }
}
