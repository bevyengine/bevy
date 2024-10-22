#[cfg(feature = "bevy_animation")]
use bevy_animation::AnimationClip;
use bevy_asset::{Handle, LoadContext};
#[cfg(feature = "bevy_animation")]
use bevy_core::Name;
use bevy_math::Mat4;
use bevy_pbr::StandardMaterial;
use bevy_render::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy_utils::{HashMap, HashSet};

#[cfg(feature = "bevy_animation")]
use crate::GltfAssetLabel;
use crate::{data_uri::DataUri, GltfError, GltfLoader, GltfLoaderSettings, GltfMesh};

use super::{ExtrasExt, MaterialExt, MeshExt, NodeExt, SkinExt};

const VALID_MIME_TYPES: &[&str] = &["application/octet-stream", "application/gltf-buffer"];

/// [`glTF`](gltf::Gltf) extension
pub trait GltfExt {
    /// Loads the raw glTF buffer data for a specific glTF file.
    async fn load_buffers(
        &self,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Vec<Vec<u8>>, GltfError>;

    #[cfg(feature = "bevy_animation")]
    #[allow(clippy::result_large_err)]
    /// Loads all animation in a [`glTF`](gltf::Gltf).
    ///
    /// Returns a list of [`Handles`](Handle) to [`AnimationClips`](AnimationClip) and a set of the animation roots.
    fn load_animations(
        &self,
        load_context: &mut LoadContext,
        buffer_data: &[Vec<u8>],
    ) -> Result<
        (
            Vec<Handle<AnimationClip>>,
            HashMap<Box<str>, Handle<AnimationClip>>,
            HashSet<usize>,
        ),
        GltfError,
    >;

    #[allow(clippy::result_large_err)]
    /// Loads all materials found on [`glTF`](gltf::Gltf).
    fn load_materials(
        &self,
        load_context: &mut LoadContext<'_>,
        settings: &GltfLoaderSettings,
    ) -> Result<
        (
            Vec<Handle<StandardMaterial>>,
            HashMap<Box<str>, Handle<StandardMaterial>>,
        ),
        GltfError,
    >;

    #[allow(clippy::result_large_err, clippy::too_many_arguments)]
    /// Load all meshes of a [`glTF`](gltf::Gltf)
    fn load_meshes(
        &self,
        loader: &GltfLoader,
        load_context: &mut LoadContext,
        settings: &GltfLoaderSettings,
        file_name: &str,
        buffer_data: &[Vec<u8>],
        materials: &[Handle<StandardMaterial>],
    ) -> Result<(Vec<Handle<GltfMesh>>, HashMap<Box<str>, Handle<GltfMesh>>), GltfError>;

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

    #[cfg(feature = "bevy_animation")]
    fn load_animation_paths(&self) -> HashMap<usize, (usize, Vec<Name>)>;
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

    #[cfg(feature = "bevy_animation")]
    #[allow(clippy::result_large_err)]
    fn load_animations(
        &self,
        load_context: &mut LoadContext,
        buffer_data: &[Vec<u8>],
    ) -> Result<
        (
            Vec<Handle<AnimationClip>>,
            HashMap<Box<str>, Handle<AnimationClip>>,
            HashSet<usize>,
        ),
        GltfError,
    > {
        use super::animation_ext::AnimationExt;

        let animation_paths = self.load_animation_paths();

        let mut animations = vec![];
        let mut named_animations = HashMap::new();
        let mut animation_roots = HashSet::new();

        for animation in self.animations() {
            let animation_clip =
                animation.load_animation(buffer_data, &animation_paths, &mut animation_roots)?;

            let handle = load_context.add_labeled_asset(
                GltfAssetLabel::Animation(animation.index()).to_string(),
                animation_clip,
            );
            if let Some(name) = animation.name() {
                named_animations.insert(name.into(), handle.clone());
            }
            animations.push(handle);
        }

        Ok((animations, named_animations, animation_roots))
    }

    #[allow(clippy::result_large_err)]
    fn load_materials(
        &self,
        load_context: &mut LoadContext<'_>,
        settings: &GltfLoaderSettings,
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
            for material in self.materials() {
                let handle = material.load_material(load_context, &self.document, false);
                if let Some(name) = material.name() {
                    named_materials.insert(name.into(), handle.clone());
                }
                materials.push(handle);
            }
        }

        Ok((materials, named_materials))
    }

    #[allow(clippy::result_large_err, clippy::too_many_arguments)]
    /// Load all meshes of a [`glTF`](gltf::Gltf)
    fn load_meshes(
        &self,
        loader: &GltfLoader,
        load_context: &mut LoadContext,
        settings: &GltfLoaderSettings,
        file_name: &str,
        buffer_data: &[Vec<u8>],
        materials: &[Handle<StandardMaterial>],
    ) -> Result<(Vec<Handle<GltfMesh>>, HashMap<Box<str>, Handle<GltfMesh>>), GltfError> {
        let (meshes_on_skinned_nodes, meshes_on_non_skinned_nodes) = self.load_meshes_on_nodes();

        let mut meshes = vec![];
        let mut named_meshes = HashMap::default();
        for gltf_mesh in self.meshes() {
            let primitives = gltf_mesh.load_primitives(
                load_context,
                loader,
                settings,
                file_name,
                buffer_data,
                &meshes_on_skinned_nodes,
                &meshes_on_non_skinned_nodes,
                materials,
            )?;

            let mesh = GltfMesh::new(&gltf_mesh, primitives, gltf_mesh.extras().get());

            let handle = load_context.add_labeled_asset(mesh.asset_label().to_string(), mesh);
            if let Some(name) = gltf_mesh.name() {
                named_meshes.insert(name.into(), handle.clone());
            }
            meshes.push(handle);
        }

        Ok((meshes, named_meshes))
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

    #[cfg(feature = "bevy_animation")]
    fn load_animation_paths(&self) -> HashMap<usize, (usize, Vec<Name>)> {
        let mut paths = HashMap::new();
        for scene in self.scenes() {
            for node in scene.nodes() {
                let root_index = node.index();
                node.paths_recur(&[], &mut paths, root_index, &mut HashSet::new());
            }
        }
        paths
    }
}
