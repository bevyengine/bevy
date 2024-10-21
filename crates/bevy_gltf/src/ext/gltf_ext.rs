use bevy_asset::{Handle, LoadContext};
use bevy_math::Mat4;
use bevy_render::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy_utils::HashSet;

use crate::{data_uri::DataUri, GltfError};

use super::{MaterialExt, NodeExt, SkinExt};

const VALID_MIME_TYPES: &[&str] = &["application/octet-stream", "application/gltf-buffer"];

/// [`glTF`](gltf::Gltf) extension
pub trait GltfExt {
    /// Loads the raw glTF buffer data for a specific glTF file.
    async fn load_buffers(
        &self,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Vec<Vec<u8>>, GltfError>;

    /// Load [`SkinnedMeshInverseBindposes`] for all [`Skin`](gltf::Skin)
    /// in [`glTF`](gltf::Gltf).
    fn inverse_bind_poses(
        &self,
        load_context: &mut LoadContext,
        buffer_data: &[Vec<u8>],
    ) -> Vec<Handle<SkinnedMeshInverseBindposes>>;

    /// Get index of [`Meshs`](gltf::Mesh) that are on skinned or non-skinned [`Nodes`](gltf::Node)
    fn load_meshes_on_nodes(&self) -> (HashSet<usize>, HashSet<usize>);

    /// Get index of [`Textures`](gltf::texture::Texture) that are used by
    /// a [`Material`](gltf::Material)
    fn textures_used_by_materials(&self) -> HashSet<usize>;
}

impl GltfExt for gltf::Gltf {
    async fn load_buffers(
        &self,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Vec<Vec<u8>>, GltfError> {
        let mut buffer_data = Vec::new();
        for buffer in self.buffers() {
            match buffer.source() {
                gltf::buffer::Source::Uri(uri) => {
                    let uri = percent_encoding::percent_decode_str(uri)
                        .decode_utf8()
                        .unwrap();
                    let uri = uri.as_ref();
                    let buffer_bytes = match DataUri::parse(uri) {
                        Ok(data_uri) if VALID_MIME_TYPES.contains(&data_uri.mime_type) => {
                            data_uri.decode()?
                        }
                        Ok(_) => return Err(GltfError::BufferFormatUnsupported),
                        Err(()) => {
                            // TODO: Remove this and add dep
                            let buffer_path = load_context.path().parent().unwrap().join(uri);
                            load_context.read_asset_bytes(buffer_path).await?
                        }
                    };
                    buffer_data.push(buffer_bytes);
                }
                gltf::buffer::Source::Bin => {
                    if let Some(blob) = self.blob.as_deref() {
                        buffer_data.push(blob.into());
                    } else {
                        return Err(GltfError::MissingBlob);
                    }
                }
            }
        }

        Ok(buffer_data)
    }

    fn inverse_bind_poses(
        &self,
        load_context: &mut LoadContext,
        buffer_data: &[Vec<u8>],
    ) -> Vec<Handle<SkinnedMeshInverseBindposes>> {
        self.skins()
            .map(|gltf_skin| {
                let reader = gltf_skin.reader(|buffer| Some(&buffer_data[buffer.index()]));
                let local_to_bone_bind_matrices: Vec<Mat4> = reader
                    .read_inverse_bind_matrices()
                    .unwrap()
                    .map(|mat| Mat4::from_cols_array_2d(&mat))
                    .collect();

                load_context.add_labeled_asset(
                    gltf_skin.inverse_bind_matrices_label().to_string(),
                    SkinnedMeshInverseBindposes::from(local_to_bone_bind_matrices),
                )
            })
            .collect()
    }

    fn load_meshes_on_nodes(&self) -> (HashSet<usize>, HashSet<usize>) {
        let mut meshes_on_skinned_nodes = HashSet::default();
        let mut meshes_on_non_skinned_nodes = HashSet::default();

        for gltf_node in self.nodes() {
            if gltf_node.is_skinned() {
                if let Some(mesh_index) = gltf_node.mesh_index() {
                    meshes_on_skinned_nodes.insert(mesh_index);
                }
            } else if let Some(mesh_index) = gltf_node.mesh_index() {
                meshes_on_non_skinned_nodes.insert(mesh_index);
            }
        }

        (meshes_on_skinned_nodes, meshes_on_non_skinned_nodes)
    }

    fn textures_used_by_materials(&self) -> HashSet<usize> {
        let mut used_textures = HashSet::default();

        for material in self.materials() {
            if let Some(normal_texture_index) = material.normal_texture_index() {
                used_textures.insert(normal_texture_index);
            }
            if let Some(occlusion_texture_index) = material.occlusion_texture_index() {
                used_textures.insert(occlusion_texture_index);
            }
            if let Some(metallic_roughness_texture_index) =
                material.metallic_roughness_texture_index()
            {
                used_textures.insert(metallic_roughness_texture_index);
            }

            #[cfg(feature = "pbr_anisotropy_texture")]
            if let Some(texture_index) =
                material.extension_texture_index("KHR_materials_anisotropy", "anisotropyTexture")
            {
                used_textures.insert(texture_index);
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
                    used_textures.insert(texture_index);
                }
            }
        }

        used_textures
    }
}
