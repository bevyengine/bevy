mod anisotropy_extension;
mod clearcoat_extension;
mod ext;

use bevy_asset::{Handle, LoadContext};
use bevy_color::{Color, LinearRgba};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_image::Image;
use bevy_math::Affine2;
use bevy_pbr::{StandardMaterial, UvChannel};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_render::{alpha::AlphaMode, render_resource::Face};
use bevy_utils::{tracing::warn, HashMap};
use ext::TexInfoExt;

// #[cfg(feature = "pbr_multi_layer_material_textures")]
// use crate::ext::MaterialExt;
use crate::{data_uri::DataUri, GltfError, GltfLoaderSettings};

use super::GltfAssetLabel;

/// A Pbr Material defined in a [`glTF`](gltf::Gltf).
#[derive(Debug, Clone)]
pub struct GltfMaterial;

impl GltfMaterial {
    #[allow(clippy::result_large_err)]
    /// Loads all materials found on [`glTF`](gltf::Gltf).
    pub(crate) fn load_materials(
        load_context: &mut LoadContext<'_>,
        settings: &GltfLoaderSettings,
        gltf: &gltf::Gltf,
    ) -> Result<
        (
            Vec<Handle<StandardMaterial>>,
            HashMap<Box<str>, Handle<StandardMaterial>>,
        ),
        GltfError,
    > {
        let mut materials = vec![];
        let mut named_materials = HashMap::new();

        // Only include materials in the output if they're set to be retained in the MAIN_WORLD and/or RENDER_WORLD by the load_materials flag
        if !settings.load_materials.is_empty() {
            // NOTE: materials must be loaded after textures because image load() calls will
            // happen before load_with_settings, preventing is_srgb from being set properly
            for material in gltf.materials() {
                let handle = Self::load_material(&material, load_context, &gltf.document, false);
                if let Some(name) = material.name() {
                    named_materials.insert(name.into(), handle.clone());
                }
                materials.push(handle);
            }
        }

        Ok((materials, named_materials))
    }

    /// Loads a glTF material as a bevy [`StandardMaterial`] and returns it.
    pub fn load_material(
        material: &gltf::Material,
        load_context: &mut LoadContext,
        document: &gltf::Document,
        is_scale_inverted: bool,
    ) -> Handle<StandardMaterial> {
        let material_label = Self::material_label(material, is_scale_inverted);
        load_context.labeled_asset_scope(material_label, |load_context| {
            let pbr = material.pbr_metallic_roughness();

            // TODO: handle missing label handle errors here?
            let color = pbr.base_color_factor();
            let uv_transform = pbr
                .base_color_texture()
                .and_then(|info| {
                    info.texture_transform()
                        .map(Self::convert_texture_transform_to_affine2)
                })
                .unwrap_or_default();
            let (base_color_channel, base_color_texture) = Self::extract_channel_and_texture(
                load_context,
                material,
                pbr.base_color_texture(),
                "base color",
                None,
            );

            let (normal_map_channel, normal_map_texture) = Self::extract_channel_and_texture(
                load_context,
                material,
                material.normal_texture(),
                "normal map",
                None,
            );

            let (metallic_roughness_channel, metallic_roughness_texture) =
                Self::extract_channel_and_texture(
                    load_context,
                    material,
                    pbr.metallic_roughness_texture(),
                    "metallic/roughness",
                    None,
                );

            // TODO: handle occlusion_texture.strength() (a scalar multiplier for occlusion strength)
            let (occlusion_channel, occlusion_texture) = Self::extract_channel_and_texture(
                load_context,
                material,
                material.occlusion_texture(),
                "occlusion",
                None,
            );

            // TODO: handle occlusion_texture.strength() (a scalar multiplier for occlusion strength)
            let emissive = material.emissive_factor();
            let (emissive_channel, emissive_texture) = Self::extract_channel_and_texture(
                load_context,
                material,
                material.emissive_texture(),
                "emissive",
                Some(uv_transform),
            );

            #[cfg(feature = "pbr_transmission_textures")]
            let (
                specular_transmission,
                specular_transmission_channel,
                specular_transmission_texture,
            ) = if let Some(transmission) = material.transmission() {
                let (specular_transmission_channel, transmission_texture) =
                    Self::extract_channel_and_texture(
                        load_context,
                        material,
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
            let specular_transmission = material
                .transmission()
                .map_or(0.0, |transmission| transmission.transmission_factor());

            #[cfg(feature = "pbr_transmission_textures")]
            let (
                thickness,
                thickness_channel,
                thickness_texture,
                attenuation_distance,
                attenuation_color,
            ) = if let Some(volume) = material.volume() {
                let (thickness_channel, thickness_texture) = Self::extract_channel_and_texture(
                    load_context,
                    material,
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
                material
                    .volume()
                    .map_or((0.0, f32::INFINITY, [1.0, 1.0, 1.0]), |volume| {
                        (
                            volume.thickness_factor(),
                            volume.attenuation_distance(),
                            volume.attenuation_color(),
                        )
                    });

            let ior = material.ior().unwrap_or(1.5);

            // Parse the `KHR_materials_clearcoat` extension data if necessary.
            let clearcoat =
                clearcoat_extension::ClearcoatExtension::parse(load_context, document, material)
                    .unwrap_or_default();

            // Parse the `KHR_materials_anisotropy` extension data if necessary.
            let anisotropy =
                anisotropy_extension::AnisotropyExtension::parse(load_context, document, material)
                    .unwrap_or_default();

            // We need to operate in the Linear color space and be willing to exceed 1.0 in our channels
            let base_emissive = LinearRgba::rgb(emissive[0], emissive[1], emissive[2]);
            let emissive = base_emissive * material.emissive_strength().unwrap_or(1.0);

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
                double_sided: material.double_sided(),
                cull_mode: if material.double_sided() {
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
                unlit: material.unlit(),
                alpha_mode: Self::alpha_mode(material),
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

    fn extract_channel_and_texture(
        load_context: &mut LoadContext,
        material: &gltf::Material,
        info_opt: Option<impl TexInfoExt>,
        texture_kind: &str,
        warn_on_differing: Option<Affine2>,
    ) -> (UvChannel, Option<Handle<Image>>) {
        if let Some(info) = info_opt {
            if let Some(uv_transform) = warn_on_differing {
                Self::warn_on_differing_texture_transforms(
                    material,
                    &info,
                    uv_transform,
                    texture_kind,
                );
            }
            (
                Self::get_uv_channel(material, texture_kind, info.tex_coord()),
                Some(Self::get_texture_from_asset_label(
                    load_context,
                    &info.texture(),
                )),
            )
        } else {
            (UvChannel::default(), None)
        }
    }

    fn alpha_mode(material: &gltf::Material) -> AlphaMode {
        match material.alpha_mode() {
            gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
            gltf::material::AlphaMode::Mask => {
                AlphaMode::Mask(material.alpha_cutoff().unwrap_or(0.5))
            }
            gltf::material::AlphaMode::Blend => AlphaMode::Blend,
        }
    }

    fn get_uv_channel(material: &gltf::Material, texture_kind: &str, tex_coord: u32) -> UvChannel {
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
    fn material_label(material: &gltf::Material, is_scale_inverted: bool) -> String {
        if let Some(index) = material.index() {
            GltfAssetLabel::Material {
                index,
                is_scale_inverted,
            }
            .to_string()
        } else {
            GltfAssetLabel::DefaultMaterial.to_string()
        }
    }

    fn get_texture_from_asset_label(
        load_context: &mut LoadContext,
        texture: &gltf::Texture,
    ) -> Handle<Image> {
        match texture.source().source() {
            gltf::image::Source::View { .. } => {
                load_context.get_label_handle(GltfAssetLabel::Texture(texture.index()).to_string())
            }
            gltf::image::Source::Uri { uri, .. } => {
                let uri = percent_encoding::percent_decode_str(uri)
                    .decode_utf8()
                    .unwrap();
                let uri = uri.as_ref();
                if let Ok(_data_uri) = DataUri::parse(uri) {
                    load_context
                        .get_label_handle(GltfAssetLabel::Texture(texture.index()).to_string())
                } else {
                    let parent = load_context.path().parent().unwrap();
                    let image_path = parent.join(uri);
                    load_context.load(image_path)
                }
            }
        }
    }

    #[cfg(any(
        feature = "pbr_anisotropy_texture",
        feature = "pbr_multi_layer_material_textures"
    ))]
    fn texture_handle_from_info(
        load_context: &mut LoadContext,
        document: &gltf::Document,
        texture_info: &gltf::json::texture::Info,
    ) -> Handle<Image> {
        let texture = document
            .textures()
            .nth(texture_info.index.value())
            .expect("Texture info references a nonexistent texture");
        Self::get_texture_from_asset_label(load_context, &texture)
    }

    fn convert_texture_transform_to_affine2(
        texture_transform: gltf::texture::TextureTransform,
    ) -> Affine2 {
        Affine2::from_scale_angle_translation(
            texture_transform.scale().into(),
            -texture_transform.rotation(),
            texture_transform.offset().into(),
        )
    }

    fn warn_on_differing_texture_transforms(
        material: &gltf::Material,
        info: &impl TexInfoExt,
        texture_transform: Affine2,
        texture_kind: &str,
    ) {
        let has_differing_texture_transform = info
            .texture_transform()
            .map(Self::convert_texture_transform_to_affine2)
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
            warn!(
            "Only texture transforms on base color textures are supported, but {material_name} ({material_index}) \
            has a texture transform on {texture_name} (index {}), which will be ignored.", info.texture().index()
        );
        }
    }
}

/// Additional untyped data that can be present on most glTF types at the material level.
///
/// See [the relevant glTF specification section](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-extras).
#[derive(Clone, Debug, Reflect, Default, Component)]
#[reflect(Component, Default, Debug)]
pub struct GltfMaterialExtras {
    /// Content of the extra data.
    pub value: String,
}

/// The material name of a glTF primitive.
///
/// See [the relevant glTF specification section](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-material).
#[derive(Clone, Debug, Reflect, Default, Component)]
#[reflect(Component)]
pub struct GltfMaterialName(pub String);
