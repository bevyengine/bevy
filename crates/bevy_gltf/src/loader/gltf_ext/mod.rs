//! Methods to access information from [`gltf`] types

pub mod material;
pub mod mesh;
pub mod scene;
pub mod texture;

use bevy_platform::collections::HashSet;

use fixedbitset::FixedBitSet;
use gltf::{Document, Gltf};

use super::GltfError;

use self::{material::extension_texture_index, scene::check_is_part_of_cycle};

#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(
        clippy::result_large_err,
        reason = "need to be signature compatible with `load_gltf`"
    )
)]
/// Checks all glTF nodes for cycles, starting at the scene root.
pub(crate) fn check_for_cycles(gltf: &Gltf) -> Result<(), GltfError> {
    // Initialize with the scene roots.
    let mut roots = FixedBitSet::with_capacity(gltf.nodes().len());
    for root in gltf.scenes().flat_map(|scene| scene.nodes()) {
        roots.insert(root.index());
    }

    // Check each one.
    let mut visited = FixedBitSet::with_capacity(gltf.nodes().len());
    for root in roots.ones() {
        let Some(node) = gltf.nodes().nth(root) else {
            unreachable!("Index of a root node should always exist.");
        };
        check_is_part_of_cycle(&node, &mut visited)?;
    }

    Ok(())
}

pub(crate) fn get_linear_textures(document: &Document) -> HashSet<usize> {
    let mut linear_textures = HashSet::default();

    for material in document.materials() {
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
            extension_texture_index(&material, "KHR_materials_anisotropy", "anisotropyTexture")
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
                extension_texture_index(&material, "KHR_materials_clearcoat", texture_field_name)
            {
                linear_textures.insert(texture_index);
            }
        }
    }

    linear_textures
}
