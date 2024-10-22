use bevy_asset::{Handle, LoadContext};
use bevy_color::{Color, LinearRgba};
use bevy_image::Image;
use bevy_math::Affine2;
use bevy_pbr::{StandardMaterial, UvChannel};
use bevy_render::{alpha::AlphaMode, render_resource::Face};
use bevy_utils::tracing::warn;

use crate::GltfAssetLabel;

use super::{TextureExt, TextureInfoExt, TextureTransformExt};

#[cfg(any(
    feature = "pbr_anisotropy_texture",
    feature = "pbr_multi_layer_material_textures"
))]
use super::JsonTextureInfoExt;

/// [`Material`](gltf::Material) extension
pub trait MaterialExt {
    /// Loads a glTF material as a bevy [`StandardMaterial`] and returns it.
    fn load_material(
        &self,
        load_context: &mut LoadContext,
        document: &gltf::Document,
        is_scale_inverted: bool,
    ) -> Handle<StandardMaterial>;

    /// Returns the label for the `material`.
    fn to_label(&self, is_scale_inverted: bool) -> GltfAssetLabel;

    /// Check if [`Material`](gltf::Material) needs tangents
    fn needs_tangents(&self) -> bool;

    /// Get the index of the normal texture
    fn normal_texture_index(&self) -> Option<usize>;

    /// Get the index of the occlusion texture
    fn occlusion_texture_index(&self) -> Option<usize>;

    /// Get the index of the occlusion texture
    fn metallic_roughness_texture_index(&self) -> Option<usize>;

    #[cfg(any(
        feature = "pbr_anisotropy_texture",
        feature = "pbr_multi_layer_material_textures"
    ))]
    /// Returns the index (within the `textures` array) of the texture with the
    /// given field name in the data for the material extension with the given name,
    /// if there is one.
    fn extension_texture_index(
        &self,
        extension_name: &str,
        texture_fild_name: &str,
    ) -> Option<usize>;

    fn alpha_mode(&self) -> AlphaMode;

    fn get_uv_channel(&self, texture_kind: &str, tex_coord: u32) -> UvChannel;

    /// Returns the label for the `material`.
    fn material_label(&self, is_scale_inverted: bool) -> String;

    fn extract_channel_and_texture(
        &self,
        load_context: &mut LoadContext,
        info_opt: Option<impl TextureInfoExt>,
        texture_kind: &str,
        warn_on_differing: Option<Affine2>,
    ) -> (UvChannel, Option<Handle<Image>>);

    fn warn_on_differing_texture_transforms(
        &self,
        info: &impl TextureInfoExt,
        texture_transform: Affine2,
        texture_kind: &str,
    );

    #[allow(unused_variables)]
    fn read_anisotropy_extension(
        &self,
        load_context: &mut LoadContext,
        document: &gltf::Document,
    ) -> Option<AnisotropyExtension>;

    #[allow(unused_variables)]
    fn read_clearcoat_extension(
        &self,
        load_context: &mut LoadContext,
        document: &gltf::Document,
    ) -> Option<ClearcoatExtension>;
}

impl MaterialExt for gltf::Material<'_> {
    /// Loads a glTF material as a bevy [`StandardMaterial`] and returns it.
    fn load_material(
        &self,
        load_context: &mut LoadContext,
        document: &gltf::Document,
        is_scale_inverted: bool,
    ) -> Handle<StandardMaterial> {
        let material_label = self.material_label(is_scale_inverted);
        load_context.labeled_asset_scope(material_label, |load_context| {
            let pbr = self.pbr_metallic_roughness();

            // TODO: handle missing label handle errors here?
            let color = pbr.base_color_factor();
            let uv_transform = pbr
                .base_color_texture()
                .and_then(|info| {
                    info.texture_transform()
                        .map(|transform| transform.convert_texture_transform_to_affine2())
                })
                .unwrap_or_default();
            let (base_color_channel, base_color_texture) = self.extract_channel_and_texture(
                load_context,
                pbr.base_color_texture(),
                "base color",
                None,
            );

            let (normal_map_channel, normal_map_texture) = self.extract_channel_and_texture(
                load_context,
                self.normal_texture(),
                "normal map",
                None,
            );

            let (metallic_roughness_channel, metallic_roughness_texture) = self
                .extract_channel_and_texture(
                    load_context,
                    pbr.metallic_roughness_texture(),
                    "metallic/roughness",
                    None,
                );

            // TODO: handle occlusion_texture.strength() (a scalar multiplier for occlusion strength)
            let (occlusion_channel, occlusion_texture) = self.extract_channel_and_texture(
                load_context,
                self.occlusion_texture(),
                "occlusion",
                None,
            );

            // TODO: handle occlusion_texture.strength() (a scalar multiplier for occlusion strength)
            let emissive = self.emissive_factor();
            let (emissive_channel, emissive_texture) = self.extract_channel_and_texture(
                load_context,
                self.emissive_texture(),
                "emissive",
                Some(uv_transform),
            );

            #[cfg(feature = "pbr_transmission_textures")]
            let (
                specular_transmission,
                specular_transmission_channel,
                specular_transmission_texture,
            ) = if let Some(transmission) = self.transmission() {
                let (specular_transmission_channel, transmission_texture) = self
                    .extract_channel_and_texture(
                        load_context,
                        transmission.transmission_texture(),
                        "specular/transmission",
                        None,
                    );
                (
                    transmission.transmission_factor(),
                    specular_transmission_channel,
                    transmission_texture,
                )
            } else {
                (0.0, UvChannel::Uv0, None)
            };

            #[cfg(not(feature = "pbr_transmission_textures"))]
            let specular_transmission = self
                .transmission()
                .map_or(0.0, |transmission| transmission.transmission_factor());

            #[cfg(feature = "pbr_transmission_textures")]
            let (
                thickness,
                thickness_channel,
                thickness_texture,
                attenuation_distance,
                attenuation_color,
            ) = if let Some(volume) = self.volume() {
                let (thickness_channel, thickness_texture) = self.extract_channel_and_texture(
                    load_context,
                    volume.thickness_texture(),
                    "thickness",
                    None,
                );
                (
                    volume.thickness_factor(),
                    thickness_channel,
                    thickness_texture,
                    volume.attenuation_distance(),
                    volume.attenuation_color(),
                )
            } else {
                (0.0, UvChannel::Uv0, None, f32::INFINITY, [1.0, 1.0, 1.0])
            };

            #[cfg(not(feature = "pbr_transmission_textures"))]
            let (thickness, attenuation_distance, attenuation_color) =
                self.volume()
                    .map_or((0.0, f32::INFINITY, [1.0, 1.0, 1.0]), |volume| {
                        (
                            volume.thickness_factor(),
                            volume.attenuation_distance(),
                            volume.attenuation_color(),
                        )
                    });

            let ior = self.ior().unwrap_or(1.5);

            // Parse the `KHR_materials_clearcoat` extension data if necessary.
            let clearcoat = self
                .read_clearcoat_extension(load_context, document)
                .unwrap_or_default();

            // Parse the `KHR_materials_anisotropy` extension data if necessary.
            let anisotropy = self
                .read_anisotropy_extension(load_context, document)
                .unwrap_or_default();

            // We need to operate in the Linear color space and be willing to exceed 1.0 in our channels
            let base_emissive = LinearRgba::rgb(emissive[0], emissive[1], emissive[2]);
            let emissive = base_emissive * self.emissive_strength().unwrap_or(1.0);

            StandardMaterial {
                base_color: Color::linear_rgba(color[0], color[1], color[2], color[3]),
                base_color_channel,
                base_color_texture,
                perceptual_roughness: pbr.roughness_factor(),
                metallic: pbr.metallic_factor(),
                metallic_roughness_channel,
                metallic_roughness_texture,
                normal_map_channel,
                normal_map_texture,
                double_sided: self.double_sided(),
                cull_mode: if self.double_sided() {
                    None
                } else if is_scale_inverted {
                    Some(Face::Front)
                } else {
                    Some(Face::Back)
                },
                occlusion_channel,
                occlusion_texture,
                emissive,
                emissive_channel,
                emissive_texture,
                specular_transmission,
                #[cfg(feature = "pbr_transmission_textures")]
                specular_transmission_channel,
                #[cfg(feature = "pbr_transmission_textures")]
                specular_transmission_texture,
                thickness,
                #[cfg(feature = "pbr_transmission_textures")]
                thickness_channel,
                #[cfg(feature = "pbr_transmission_textures")]
                thickness_texture,
                ior,
                attenuation_distance,
                attenuation_color: Color::linear_rgb(
                    attenuation_color[0],
                    attenuation_color[1],
                    attenuation_color[2],
                ),
                unlit: self.unlit(),
                alpha_mode: MaterialExt::alpha_mode(self),
                uv_transform,
                clearcoat: clearcoat.clearcoat_factor.unwrap_or_default() as f32,
                clearcoat_perceptual_roughness: clearcoat
                    .clearcoat_roughness_factor
                    .unwrap_or_default() as f32,
                #[cfg(feature = "pbr_multi_layer_material_textures")]
                clearcoat_channel: clearcoat.clearcoat_channel,
                #[cfg(feature = "pbr_multi_layer_material_textures")]
                clearcoat_texture: clearcoat.clearcoat_texture,
                #[cfg(feature = "pbr_multi_layer_material_textures")]
                clearcoat_roughness_channel: clearcoat.clearcoat_roughness_channel,
                #[cfg(feature = "pbr_multi_layer_material_textures")]
                clearcoat_roughness_texture: clearcoat.clearcoat_roughness_texture,
                #[cfg(feature = "pbr_multi_layer_material_textures")]
                clearcoat_normal_channel: clearcoat.clearcoat_normal_channel,
                #[cfg(feature = "pbr_multi_layer_material_textures")]
                clearcoat_normal_texture: clearcoat.clearcoat_normal_texture,
                anisotropy_strength: anisotropy.anisotropy_strength.unwrap_or_default() as f32,
                anisotropy_rotation: anisotropy.anisotropy_rotation.unwrap_or_default() as f32,
                #[cfg(feature = "pbr_anisotropy_texture")]
                anisotropy_channel: anisotropy.anisotropy_channel,
                #[cfg(feature = "pbr_anisotropy_texture")]
                anisotropy_texture: anisotropy.anisotropy_texture,
                ..Default::default()
            }
        })
    }

    fn to_label(&self, is_scale_inverted: bool) -> GltfAssetLabel {
        if let Some(index) = self.index() {
            GltfAssetLabel::Material {
                index,
                is_scale_inverted,
            }
        } else {
            GltfAssetLabel::DefaultMaterial
        }
    }

    fn needs_tangents(&self) -> bool {
        if self.normal_texture().is_some() {
            return true;
        }

        #[cfg(feature = "pbr_multi_layer_material_textures")]
        if self
            .extension_texture_index("KHR_materials_clearcoat", "clearcoatNormalTexture")
            .is_some()
        {
            return true;
        }

        false
    }

    fn normal_texture_index(&self) -> Option<usize> {
        self.normal_texture()
            .map(|normal_texture_info| normal_texture_info.texture().index())
    }

    fn occlusion_texture_index(&self) -> Option<usize> {
        self.occlusion_texture()
            .map(|occlusion_texture_info| occlusion_texture_info.texture().index())
    }

    fn metallic_roughness_texture_index(&self) -> Option<usize> {
        self.pbr_metallic_roughness()
            .metallic_roughness_texture()
            .map(|metallic_roughness_texture_info| {
                metallic_roughness_texture_info.texture().index()
            })
    }

    #[cfg(any(
        feature = "pbr_anisotropy_texture",
        feature = "pbr_multi_layer_material_textures"
    ))]
    fn extension_texture_index(
        &self,
        extension_name: &str,
        texture_field_name: &str,
    ) -> Option<usize> {
        Some(
            serde_json::value::from_value::<gltf::json::texture::Info>(
                self.extensions()?
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

    fn alpha_mode(&self) -> AlphaMode {
        match self.alpha_mode() {
            gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
            gltf::material::AlphaMode::Mask => AlphaMode::Mask(self.alpha_cutoff().unwrap_or(0.5)),
            gltf::material::AlphaMode::Blend => AlphaMode::Blend,
        }
    }

    fn get_uv_channel(&self, texture_kind: &str, tex_coord: u32) -> UvChannel {
        match tex_coord {
            0 => UvChannel::Uv0,
            1 => UvChannel::Uv1,
            _ => {
                let material_name = self
                    .name()
                    .map(|n| format!("the material \"{n}\""))
                    .unwrap_or_else(|| "an unnamed material".to_string());
                let material_index = self
                    .index()
                    .map(|i| format!("index {i}"))
                    .unwrap_or_else(|| "default".to_string());
                warn!(
            "Only 2 UV Channels are supported, but {material_name} ({material_index}) \
            has the TEXCOORD attribute {} on texture kind {texture_kind}, which will fallback to 0.",
            tex_coord,
        );
                UvChannel::Uv0
            }
        }
    }

    /// Returns the label for the `material`.
    fn material_label(&self, is_scale_inverted: bool) -> String {
        if let Some(index) = self.index() {
            GltfAssetLabel::Material {
                index,
                is_scale_inverted,
            }
            .to_string()
        } else {
            GltfAssetLabel::DefaultMaterial.to_string()
        }
    }

    fn extract_channel_and_texture(
        &self,
        load_context: &mut LoadContext,
        info_opt: Option<impl TextureInfoExt>,
        texture_kind: &str,
        warn_on_differing: Option<Affine2>,
    ) -> (UvChannel, Option<Handle<Image>>) {
        if let Some(info) = info_opt {
            if let Some(uv_transform) = warn_on_differing {
                self.warn_on_differing_texture_transforms(&info, uv_transform, texture_kind);
            }
            (
                self.get_uv_channel(texture_kind, info.tex_coord()),
                Some(info.texture().get_texture_from_asset_label(load_context)),
            )
        } else {
            (UvChannel::default(), None)
        }
    }

    fn warn_on_differing_texture_transforms(
        &self,
        info: &impl TextureInfoExt,
        texture_transform: Affine2,
        texture_kind: &str,
    ) {
        let has_differing_texture_transform = info
            .texture_transform()
            .map(|transform| transform.convert_texture_transform_to_affine2())
            .is_some_and(|t| t != texture_transform);
        if has_differing_texture_transform {
            let material_name = self
                .name()
                .map(|n| format!("the material \"{n}\""))
                .unwrap_or_else(|| "an unnamed material".to_string());
            let texture_name = info
                .texture()
                .name()
                .map(|n| format!("its {texture_kind} texture \"{n}\""))
                .unwrap_or_else(|| format!("its unnamed {texture_kind} texture"));
            let material_index = self
                .index()
                .map(|i| format!("index {i}"))
                .unwrap_or_else(|| "default".to_string());
            warn!(
            "Only texture transforms on base color textures are supported, but {material_name} ({material_index}) \
            has a texture transform on {texture_name} (index {}), which will be ignored.", info.texture().index()
        );
        }
    }

    #[allow(unused_variables)]
    fn read_anisotropy_extension(
        &self,
        load_context: &mut LoadContext,
        document: &gltf::Document,
    ) -> Option<AnisotropyExtension> {
        let extension = self
            .extensions()?
            .get("KHR_materials_anisotropy")?
            .as_object()?;

        #[cfg(feature = "pbr_anisotropy_texture")]
        let (anisotropy_channel, anisotropy_texture) = extension
            .get("anisotropyTexture")
            .and_then(|value| {
                serde_json::value::from_value::<gltf::json::texture::Info>(value.clone()).ok()
            })
            .map(|json_info| {
                (
                    self.get_uv_channel("anisotropy", json_info.tex_coord),
                    json_info.texture_handle_from_info(load_context, document),
                )
            })
            .unzip();

        Some(AnisotropyExtension {
            anisotropy_strength: extension
                .get("anisotropyStrength")
                .and_then(serde_json::Value::as_f64),
            anisotropy_rotation: extension
                .get("anisotropyRotation")
                .and_then(serde_json::Value::as_f64),
            #[cfg(feature = "pbr_anisotropy_texture")]
            anisotropy_channel: anisotropy_channel.unwrap_or_default(),
            #[cfg(feature = "pbr_anisotropy_texture")]
            anisotropy_texture,
        })
    }

    #[allow(unused_variables)]
    fn read_clearcoat_extension(
        &self,
        load_context: &mut LoadContext,
        document: &gltf::Document,
    ) -> Option<ClearcoatExtension> {
        let extension = self
            .extensions()?
            .get("KHR_materials_clearcoat")?
            .as_object()?;

        #[cfg(feature = "pbr_multi_layer_material_textures")]
        let (clearcoat_channel, clearcoat_texture) = extension
            .get("clearcoatTexture")
            .and_then(|value| {
                serde_json::value::from_value::<gltf::json::texture::Info>(value.clone()).ok()
            })
            .map(|json_info| {
                (
                    self.get_uv_channel("clearcoat", json_info.tex_coord),
                    json_info.texture_handle_from_info(load_context, document),
                )
            })
            .unzip();

        #[cfg(feature = "pbr_multi_layer_material_textures")]
        let (clearcoat_roughness_channel, clearcoat_roughness_texture) = extension
            .get("clearcoatRoughnessTexture")
            .and_then(|value| {
                serde_json::value::from_value::<gltf::json::texture::Info>(value.clone()).ok()
            })
            .map(|json_info| {
                (
                    self.get_uv_channel("clearcoat roughness", json_info.tex_coord),
                    json_info.texture_handle_from_info(load_context, document),
                )
            })
            .unzip();

        #[cfg(feature = "pbr_multi_layer_material_textures")]
        let (clearcoat_normal_channel, clearcoat_normal_texture) = extension
            .get("clearcoatNormalTexture")
            .and_then(|value| {
                serde_json::value::from_value::<gltf::json::texture::Info>(value.clone()).ok()
            })
            .map(|json_info| {
                (
                    self.get_uv_channel("clearcoat normal", json_info.tex_coord),
                    json_info.texture_handle_from_info(load_context, document),
                )
            })
            .unzip();

        Some(ClearcoatExtension {
            clearcoat_factor: extension
                .get("clearcoatFactor")
                .and_then(serde_json::Value::as_f64),
            clearcoat_roughness_factor: extension
                .get("clearcoatRoughnessFactor")
                .and_then(serde_json::Value::as_f64),
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
