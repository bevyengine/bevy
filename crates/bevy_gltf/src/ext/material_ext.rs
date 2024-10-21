use crate::GltfAssetLabel;

/// [`Material`](gltf::Material) extension
pub trait MaterialExt {
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
}

impl MaterialExt for gltf::Material<'_> {
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
}
