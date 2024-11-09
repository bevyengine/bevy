mod mesh;
mod node;
mod primitive;
mod skin;

#[cfg(feature = "bevy_animation")]
use bevy_animation::AnimationClip;
use bevy_asset::{Asset, Handle, LoadContext};
use bevy_pbr::StandardMaterial;
use bevy_reflect::TypePath;
use bevy_scene::Scene;
use bevy_utils::HashMap;

use crate::{ext::GltfExt, GltfError, GltfLoader, GltfLoaderSettings};

pub use self::{mesh::GltfMesh, node::GltfNode, primitive::GltfPrimitive, skin::GltfSkin};

/// Representation of a loaded glTF file.
#[derive(Asset, Debug, TypePath)]
pub struct Gltf {
    /// All scenes loaded from the glTF file.
    pub scenes: Vec<Handle<Scene>>,
    /// Named scenes loaded from the glTF file.
    pub named_scenes: HashMap<Box<str>, Handle<Scene>>,
    /// All meshes loaded from the glTF file.
    pub meshes: Vec<Handle<GltfMesh>>,
    /// Named meshes loaded from the glTF file.
    pub named_meshes: HashMap<Box<str>, Handle<GltfMesh>>,
    /// All materials loaded from the glTF file.
    pub materials: Vec<Handle<StandardMaterial>>,
    /// Named materials loaded from the glTF file.
    pub named_materials: HashMap<Box<str>, Handle<StandardMaterial>>,
    /// All nodes loaded from the glTF file.
    pub nodes: Vec<Handle<GltfNode>>,
    /// Named nodes loaded from the glTF file.
    pub named_nodes: HashMap<Box<str>, Handle<GltfNode>>,
    /// All skins loaded from the glTF file.
    pub skins: Vec<Handle<GltfSkin>>,
    /// Named skins loaded from the glTF file.
    pub named_skins: HashMap<Box<str>, Handle<GltfSkin>>,
    /// Default scene to be displayed.
    pub default_scene: Option<Handle<Scene>>,
    /// All animations loaded from the glTF file.
    #[cfg(feature = "bevy_animation")]
    pub animations: Vec<Handle<AnimationClip>>,
    #[cfg(feature = "bevy_animation")]
    /// Named animations loaded from the glTF file.
    pub named_animations: HashMap<Box<str>, Handle<AnimationClip>>,
    /// The gltf root of the gltf asset, see <https://docs.rs/gltf/latest/gltf/struct.Gltf.html>. Only has a value when `GltfLoaderSettings::include_source` is true.
    pub source: Option<gltf::Gltf>,
}

impl Gltf {
    /// Loads an entire glTF file.
    pub async fn load_gltf<'a, 'b, 'c>(
        loader: &GltfLoader,
        file_name: &str,
        bytes: &'a [u8],
        load_context: &'b mut LoadContext<'c>,
        settings: &'b GltfLoaderSettings,
    ) -> Result<Gltf, GltfError> {
        let gltf = gltf::Gltf::from_slice(bytes)?;
        let buffer_data = gltf.load_buffers(load_context).await?;

        let textures_used_by_materials = gltf.textures_used_by_materials();

        // We collect handles to ensure loaded images from paths are not unloaded before they are used elsewhere
        // in the loader. This prevents "reloads", but it also prevents dropping the is_srgb context on reload.
        //
        // In theory we could store a mapping between texture.index() and handle to use
        // later in the loader when looking up handles for materials. However this would mean
        // that the material's load context would no longer track those images as dependencies.
        let _textures = gltf
            .load_textures(
                loader,
                load_context,
                settings,
                &buffer_data,
                &textures_used_by_materials,
            )
            .await?;

        let (materials, named_materials) = gltf.load_materials(load_context, settings)?;

        #[cfg(feature = "bevy_animation")]
        let (animations, named_animations, animation_roots) =
            gltf.load_animations(load_context, &buffer_data)?;

        let (meshes, named_meshes) = gltf.load_meshes(
            loader,
            load_context,
            settings,
            file_name,
            &buffer_data,
            &materials,
        )?;

        let (skins, named_skins) = gltf.load_skins(load_context, &buffer_data)?;
        let (nodes, named_nodes) = gltf.load_nodes(
            load_context,
            #[cfg(feature = "bevy_animation")]
            &animation_roots,
        )?;

        let (scenes, named_scenes) = gltf.load_scenes(
            load_context,
            settings,
            #[cfg(feature = "bevy_animation")]
            &animation_roots,
        )?;

        Ok(Gltf {
            default_scene: gltf
                .default_scene()
                .and_then(|scene| scenes.get(scene.index()))
                .cloned(),
            scenes,
            named_scenes,
            meshes,
            named_meshes,
            skins,
            named_skins,
            materials,
            named_materials,
            nodes,
            named_nodes,
            #[cfg(feature = "bevy_animation")]
            animations,
            #[cfg(feature = "bevy_animation")]
            named_animations,
            source: if settings.include_source {
                Some(gltf)
            } else {
                None
            },
        })
    }
}
