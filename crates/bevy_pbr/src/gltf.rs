use bevy_gltf::{
    extensions::{ErasedGltfExtensionHandler, GltfExtensionHandler, GltfExtensionHandlers},
    gltf, GltfAssetLabel, GltfMaterial,
};

use crate::{MeshMaterial3d, StandardMaterial, CLEARCOAT_IOR};
use bevy_app::App;
use bevy_asset::Handle;
use bevy_color::{Color, ColorToComponents, LinearRgba};
use bevy_ecs::prelude::*;

use bevy_asset::LoadContext;

pub(crate) fn add_gltf(app: &mut App) {
    #[cfg(target_family = "wasm")]
    bevy_tasks::block_on(async {
        app.world_mut()
            .resource_mut::<GltfExtensionHandlers>()
            .0
            .write()
            .await
            .push(Box::new(GltfExtensionHandlerPbr));
    });

    #[cfg(not(target_family = "wasm"))]
    app.world_mut()
        .resource_mut::<GltfExtensionHandlers>()
        .0
        .write_blocking()
        .push(Box::new(GltfExtensionHandlerPbr));
}

/// Converts a [`GltfMaterial`] to a [`StandardMaterial`]
///
/// glTF's specular and IOR parameters are converted to the OpenPBR model that
/// [`StandardMaterial`] uses, so that the reflectance at normal incidence matches glTF's, except
/// under a textured clearcoat. glTF's specular texture scales the weight, so where the texture is
/// below 1, refraction and reflection at grazing angles differ from glTF.
pub fn standard_material_from_gltf_material(material: &GltfMaterial) -> StandardMaterial {
    let (specular_weight, specular_tint) =
        specular_from_gltf(material.specular, material.specular_tint);
    // A clearcoat texture varies the clearcoat per texel, so a single IOR can't match glTF's
    // reflectance everywhere. Leaving the IOR alone keeps the uncoated texels correct.
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    let clearcoat_textured = material.clearcoat_texture.is_some();
    #[cfg(not(feature = "pbr_multi_layer_material_textures"))]
    let clearcoat_textured = false;
    let ior = if clearcoat_textured {
        material.ior
    } else {
        ior_under_clearcoat_from_gltf(material.ior, material.clearcoat)
    };
    StandardMaterial {
        base_color: material.base_color,
        base_color_channel: material.base_color_channel.clone(),
        base_color_texture: material.base_color_texture.clone(),
        emissive: material.emissive,
        emissive_channel: material.emissive_channel.clone(),
        emissive_texture: material.emissive_texture.clone(),
        perceptual_roughness: material.perceptual_roughness,
        metallic: material.metallic,
        metallic_roughness_channel: material.metallic_roughness_channel.clone(),
        metallic_roughness_texture: material.metallic_roughness_texture.clone(),
        specular_weight,
        specular_tint,
        specular_transmission: material.specular_transmission,
        #[cfg(feature = "pbr_transmission_textures")]
        specular_transmission_channel: material.specular_transmission_channel.clone(),
        #[cfg(feature = "pbr_transmission_textures")]
        specular_transmission_texture: material.specular_transmission_texture.clone(),
        thickness: material.thickness,
        #[cfg(feature = "pbr_transmission_textures")]
        thickness_channel: material.thickness_channel.clone(),
        #[cfg(feature = "pbr_transmission_textures")]
        thickness_texture: material.thickness_texture.clone(),
        ior,
        attenuation_distance: material.attenuation_distance,
        attenuation_color: material.attenuation_color,
        normal_map_channel: material.normal_map_channel.clone(),
        normal_map_texture: material.normal_map_texture.clone(),
        occlusion_channel: material.occlusion_channel.clone(),
        occlusion_texture: material.occlusion_texture.clone(),
        #[cfg(feature = "pbr_specular_textures")]
        specular_channel: material.specular_channel.clone(),
        #[cfg(feature = "pbr_specular_textures")]
        specular_texture: material.specular_texture.clone(),
        #[cfg(feature = "pbr_specular_textures")]
        specular_tint_channel: material.specular_tint_channel.clone(),
        #[cfg(feature = "pbr_specular_textures")]
        specular_tint_texture: material.specular_tint_texture.clone(),
        clearcoat: material.clearcoat,
        #[cfg(feature = "pbr_multi_layer_material_textures")]
        clearcoat_channel: material.clearcoat_channel.clone(),
        #[cfg(feature = "pbr_multi_layer_material_textures")]
        clearcoat_texture: material.clearcoat_texture.clone(),
        clearcoat_perceptual_roughness: material.clearcoat_perceptual_roughness,
        #[cfg(feature = "pbr_multi_layer_material_textures")]
        clearcoat_roughness_channel: material.clearcoat_roughness_channel.clone(),
        #[cfg(feature = "pbr_multi_layer_material_textures")]
        clearcoat_roughness_texture: material.clearcoat_roughness_texture.clone(),
        #[cfg(feature = "pbr_multi_layer_material_textures")]
        clearcoat_normal_channel: material.clearcoat_normal_channel.clone(),
        #[cfg(feature = "pbr_multi_layer_material_textures")]
        clearcoat_normal_texture: material.clearcoat_normal_texture.clone(),
        anisotropy_strength: material.anisotropy_strength,
        anisotropy_rotation: material.anisotropy_rotation,
        #[cfg(feature = "pbr_anisotropy_texture")]
        anisotropy_channel: material.anisotropy_channel.clone(),
        #[cfg(feature = "pbr_anisotropy_texture")]
        anisotropy_texture: material.anisotropy_texture.clone(),
        double_sided: material.double_sided,
        cull_mode: material.cull_mode,
        unlit: material.unlit,
        alpha_mode: material.alpha_mode,
        uv_transform: material.uv_transform,
        ..Default::default()
    }
}

/// Returns the specular weight and tint for `KHR_materials_specular`'s `specularFactor` and
/// `specularColorFactor`.
///
/// glTF's factor and color scale reflection without changing refraction, so they go into the tint,
/// which does the same. Brightening that the tint can't express goes into the weight, which also
/// changes refraction. Unlike glTF, the light the tint removes isn't added to diffuse reflection or
/// transmission.
fn specular_from_gltf(specular_factor: f32, specular_color: Color) -> (f32, Color) {
    let color = specular_factor * LinearRgba::from(specular_color).to_vec3();
    let weight = color.max_element().max(1.0);
    (weight, LinearRgba::from_vec3(color / weight).into())
}

/// Returns the IOR that gives a base layer under a clearcoat of strength `clearcoat` the
/// reflectance at normal incidence that `ior` gives it in glTF.
///
/// `KHR_materials_clearcoat` leaves the base layer's IOR relative to air, but
/// [`StandardMaterial::ior`] is relative to the clearcoat. When no IOR gives the same
/// reflectance, this returns the closest one. Refraction also uses the returned IOR.
fn ior_under_clearcoat_from_gltf(ior: f32, clearcoat: f32) -> f32 {
    // glTF only allows an IOR below 1 when it is 0, which reflects all light under any clearcoat.
    if clearcoat <= 0.0 || ior < 1.0 {
        return ior;
    }
    let c = clearcoat.min(1.0);

    // At or above the clearcoat's IOR, the relative IOR is `x * lerp(1, 1 / CLEARCOAT_IOR, c)`.
    let above = ior / (1.0 - c + c / CLEARCOAT_IOR);
    if above >= CLEARCOAT_IOR {
        return above;
    }

    // Below it, the relative IOR is `(1 - c) * x + c * CLEARCOAT_IOR / x`, with `c` below 1. Take
    // the root closest to `CLEARCOAT_IOR`, or the minimum when there's no root. The shader treats
    // IORs below 1 as their reciprocals, so the result is at least 1.
    let discriminant = ior * ior - 4.0 * (1.0 - c) * c * CLEARCOAT_IOR;
    let x = if discriminant >= 0.0 {
        (ior + discriminant.sqrt()) / (2.0 * (1.0 - c))
    } else {
        (c * CLEARCOAT_IOR / (1.0 - c)).sqrt()
    };
    x.clamp(1.0, CLEARCOAT_IOR)
}

#[derive(Default, Clone)]
struct GltfExtensionHandlerPbr;

impl GltfExtensionHandler for GltfExtensionHandlerPbr {
    fn dyn_clone(&self) -> Box<dyn ErasedGltfExtensionHandler> {
        Box::new((*self).clone())
    }
    fn on_root(
        &mut self,
        load_context: &mut LoadContext<'_>,
        _gltf: &gltf::Gltf,
        _settings: &bevy_gltf::GltfLoaderSettings,
    ) {
        // create the `StandardMaterial` for the glTF `DefaultMaterial` so
        // it can be accessed when meshes don't have materials.
        let std_label = format!("{}/std", GltfAssetLabel::DefaultMaterial);

        load_context.add_labeled_asset(
            std_label,
            standard_material_from_gltf_material(&GltfMaterial::default()),
        );
    }

    fn on_material(
        &mut self,
        load_context: &mut LoadContext<'_>,
        _gltf_material: &gltf::Material,
        _material: Handle<GltfMaterial>,
        material_asset: &GltfMaterial,
        material_label: &str,
    ) {
        let std_label = format!("{}/std", material_label);

        load_context.add_labeled_asset(
            std_label,
            standard_material_from_gltf_material(material_asset),
        );
    }

    fn on_spawn_mesh_and_material(
        &mut self,
        load_context: &mut LoadContext<'_>,
        _primitive: &gltf::Primitive,
        _mesh: &gltf::Mesh,
        _material: &gltf::Material,
        entity: &mut EntityWorldMut,
        material_label: &str,
    ) {
        let std_label = format!("{}/std", material_label);
        let handle = load_context.get_label_handle::<StandardMaterial>(std_label);

        entity.insert(MeshMaterial3d(handle));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base_layer_relative_ior;
    use bevy_math::ops::FloatPow;

    fn f0(ior: f32) -> f32 {
        ((ior - 1.0) / (ior + 1.0)).squared()
    }

    #[test]
    fn specular_from_gltf_keeps_f0() {
        assert_eq!(specular_from_gltf(1.0, Color::WHITE), (1.0, Color::WHITE));
        assert_eq!(
            specular_from_gltf(0.25, Color::WHITE),
            (1.0, Color::linear_rgb(0.25, 0.25, 0.25))
        );
        assert_eq!(
            specular_from_gltf(0.5, Color::linear_rgb(0.0, 0.0, 0.0)),
            (1.0, Color::BLACK)
        );
        assert_eq!(
            specular_from_gltf(0.5, Color::linear_rgb(2.0, 1.0, 0.5)),
            (1.0, Color::linear_rgb(1.0, 0.5, 0.25))
        );
        assert_eq!(
            specular_from_gltf(1.0, Color::linear_rgb(4.0, 2.0, 1.0)),
            (4.0, Color::linear_rgb(1.0, 0.5, 0.25))
        );
    }

    #[test]
    fn ior_under_clearcoat_from_gltf_keeps_f0() {
        assert_eq!(ior_under_clearcoat_from_gltf(1.5, 0.0), 1.5);
        assert_eq!(ior_under_clearcoat_from_gltf(0.0, 1.0), 0.0);

        for clearcoat in [0.01, 0.1, 0.25, 0.5, 0.75, 1.0] {
            for ior in [1.0, 1.1, 1.33, 1.5, 2.0, 2.42] {
                let converted = ior_under_clearcoat_from_gltf(ior, clearcoat);
                let expected = f0(ior);
                let actual = f0(base_layer_relative_ior(converted, clearcoat));
                // Some combinations have no exact match. They give the closest reflectance.
                let closest = (0..=3000)
                    .map(|i| f0(base_layer_relative_ior(i as f32 * 0.001, clearcoat)))
                    .map(|f| (f - expected).abs())
                    .fold(f32::INFINITY, f32::min);
                assert!(
                    (actual - expected).abs() <= closest + 1e-4,
                    "ior {ior}, clearcoat {clearcoat}: F0 {actual}, expected {expected}"
                );
            }
        }
    }
}
