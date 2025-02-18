use bevy_platform_support::collections::HashSet;

use gltf::Document;

use super::material::MaterialExt;

pub trait DocumentExt {
    fn get_linear_textures(&self) -> HashSet<usize>;
}

impl DocumentExt for Document {
    fn get_linear_textures(&self) -> HashSet<usize> {
        let mut linear_textures = HashSet::default();

        for material in self.materials() {
            if let Some(texture) = material.normal_texture() {
                linear_textures.insert(texture.texture().index());
            }
            if let Some(texture) = material.occlusion_texture() {
                linear_textures.insert(texture.texture().index());
            }
            if let Some(texture) = material
                .pbr_metallic_roughness()
                .metallic_roughness_texture()
            {
                linear_textures.insert(texture.texture().index());
            }
            if let Some(texture_index) =
                material.extension_texture_index("KHR_materials_anisotropy", "anisotropyTexture")
            {
                linear_textures.insert(texture_index);
            }

            // None of the clearcoat maps should be loaded as sRGB.
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            for texture_field_name in [
                "clearcoatTexture",
                "clearcoatRoughnessTexture",
                "clearcoatNormalTexture",
            ] {
                if let Some(texture_index) =
                    material.extension_texture_index("KHR_materials_clearcoat", texture_field_name)
                {
                    linear_textures.insert(texture_index);
                }
            }
        }

        linear_textures
    }
}
