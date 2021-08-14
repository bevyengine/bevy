use std::collections::HashMap;

mod loader;
pub use loader::*;

use bevy_app::prelude::*;
use bevy_asset::{AddAsset, Handle};
use bevy_pbr::prelude::StandardMaterial;
use bevy_reflect::TypeUuid;
use bevy_render::mesh::Mesh;
use bevy_scene::Scene;

/// Adds support for GLTF file loading to Apps
#[derive(Default)]
pub struct GltfPlugin;

impl Plugin for GltfPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<GltfLoader>()
            .add_asset::<Gltf>()
            .add_asset::<GltfNode>()
            .add_asset::<GltfPrimitive>()
            .add_asset::<GltfMesh>();
    }
}

#[derive(Debug, TypeUuid)]
#[uuid = "5c7d5f8a-f7b0-4e45-a09e-406c0372fea2"]
pub struct Gltf {
    pub scenes: Vec<Handle<Scene>>,
    pub named_scenes: HashMap<String, Handle<Scene>>,
    pub meshes: Vec<Handle<GltfMesh>>,
    pub named_meshes: HashMap<String, Handle<GltfMesh>>,
    pub materials: Vec<Handle<StandardMaterial>>,
    pub named_materials: HashMap<String, Handle<StandardMaterial>>,
    pub nodes: Vec<Handle<GltfNode>>,
    pub named_nodes: HashMap<String, Handle<GltfNode>>,
    pub default_scene: Option<Handle<Scene>>,
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "dad74750-1fd6-460f-ac51-0a7937563865"]
pub struct GltfNode {
    pub children: Vec<GltfNode>,
    pub mesh: Option<Handle<GltfMesh>>,
    pub transform: bevy_transform::prelude::Transform,
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "8ceaec9a-926a-4f29-8ee3-578a69f42315"]
pub struct GltfMesh {
    pub primitives: Vec<GltfPrimitive>,
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "cbfca302-82fd-41cb-af77-cab6b3d50af1"]
pub struct GltfPrimitive {
    pub mesh: Handle<Mesh>,
    pub material: Option<Handle<StandardMaterial>>,
}
