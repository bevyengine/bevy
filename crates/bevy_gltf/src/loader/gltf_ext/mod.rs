//! Extension traits for [`gltf`] types

pub mod extras;
pub mod material;
pub mod mesh;
pub mod scene;
pub mod texture;

use bevy_platform_support::collections::HashSet;

use fixedbitset::FixedBitSet;
use gltf::{Document, Gltf};

use super::GltfError;

use self::{material::MaterialExt, scene::NodeExt};

pub trait GltfExt {
    #[expect(
        clippy::result_large_err,
        reason = "need to be signature compatible with `load_gltf`"
    )]
    fn check_for_cycles(&self) -> Result<(), GltfError>;
}

pub trait DocumentExt {
    fn get_linear_textures(&self) -> HashSet<usize>;
}

impl GltfExt for Gltf {
    /// Checks all glTF nodes for cycles, starting at the scene root.
    fn check_for_cycles(&self) -> Result<(), GltfError> {
        // Initialize with the scene roots.
        let mut roots = FixedBitSet::with_capacity(self.nodes().len());
        for root in self.scenes().flat_map(|scene| scene.nodes()) {
            roots.insert(root.index());
        }

        // Check each one.
        let mut visited = FixedBitSet::with_capacity(self.nodes().len());
        for root in roots.ones() {
            let Some(node) = self.nodes().nth(root) else {
                unreachable!("Index of a root node should always exist.");
            };
            node.check_is_part_of_cycle(&mut visited)?;
        }

        Ok(())
    }
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
