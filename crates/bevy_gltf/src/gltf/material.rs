use bevy_asset::{Handle, LoadContext};
use bevy_color::{Color, LinearRgba};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_pbr::StandardMaterial;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_render::render_resource::Face;
use bevy_utils::HashMap;

use crate::{
    ext::{MaterialExt, TextureTransformExt},
    GltfError, GltfLoaderSettings,
};

#[cfg(feature = "pbr_transmission_textures")]
use bevy_pbr::UvChannel;

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
        let material_label = material.material_label(is_scale_inverted);
        load_context.labeled_asset_scope(material_label, |load_context| {
            let pbr = material.pbr_metallic_roughness();

            // TODO: handle missing label handle errors here?
            let color = pbr.base_color_factor();
            let uv_transform = pbr
                .base_color_texture()
                .and_then(|info| {
                    info.texture_transform()
                        .map(|transform| transform.convert_texture_transform_to_affine2())
                })
                .unwrap_or_default();
            let (base_color_channel, base_color_texture) = material.extract_channel_and_texture(
                load_context,
                pbr.base_color_texture(),
                "base color",
                None,
            );

            let (normal_map_channel, normal_map_texture) = material.extract_channel_and_texture(
                load_context,
                material.normal_texture(),
                "normal map",
                None,
            );

            let (metallic_roughness_channel, metallic_roughness_texture) = material
                .extract_channel_and_texture(
                    load_context,
                    pbr.metallic_roughness_texture(),
                    "metallic/roughness",
                    None,
                );

            // TODO: handle occlusion_texture.strength() (a scalar multiplier for occlusion strength)
            let (occlusion_channel, occlusion_texture) = material.extract_channel_and_texture(
                load_context,
                material.occlusion_texture(),
                "occlusion",
                None,
            );

            // TODO: handle occlusion_texture.strength() (a scalar multiplier for occlusion strength)
            let emissive = material.emissive_factor();
            let (emissive_channel, emissive_texture) = material.extract_channel_and_texture(
                load_context,
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
                let (specular_transmission_channel, transmission_texture) = material
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
                let (thickness_channel, thickness_texture) = material.extract_channel_and_texture(
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
            let clearcoat = material
                .read_clearcoat_extension(load_context, document)
                .unwrap_or_default();

            // Parse the `KHR_materials_anisotropy` extension data if necessary.
            let anisotropy = material
                .read_anisotropy_extension(load_context, document)
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
                alpha_mode: MaterialExt::alpha_mode(material),
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
