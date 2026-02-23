use bevy_gltf::{
    extensions::{GltfExtensionHandler, GltfExtensionHandlers},
    gltf, GltfAssetLabel, GltfMaterial,
};

use crate::{MeshMaterial3d, StandardMaterial};
use bevy_app::App;
use bevy_asset::Handle;
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
pub fn standard_material_from_gltf_material(material: &GltfMaterial) -> StandardMaterial {
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
        reflectance: material.reflectance,
        specular_tint: material.specular_tint,
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
        ior: material.ior,
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

#[derive(Default, Clone)]
struct GltfExtensionHandlerPbr;

impl GltfExtensionHandler for GltfExtensionHandlerPbr {
    fn dyn_clone(&self) -> Box<dyn GltfExtensionHandler> {
        Box::new((*self).clone())
    }
    fn on_root(&mut self, load_context: &mut LoadContext<'_>, _gltf: &gltf::Gltf) {
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
