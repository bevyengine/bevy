mod loader;
pub use loader::*;

use bevy_app::prelude::*;
use bevy_asset::AddAsset;

/// Adds support for GLTF file loading to Apps
#[derive(Default)]
pub struct GltfPlugin;

impl Plugin for GltfPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_asset_loader::<GltfLoader>()
            .add_asset::<GltfNode>()
            .add_asset::<GltfMesh>();
    }
}

#[derive(Debug, Clone, bevy_reflect::TypeUuid)]
#[uuid = "dad74750-1fd6-460f-ac51-0a7937563865"]
pub struct GltfNode {
    pub children: Vec<usize>,
    pub mesh: Option<usize>,
    pub transform: bevy_transform::prelude::Transform,
}

#[derive(Debug, Clone, bevy_reflect::TypeUuid)]
#[uuid = "8ceaec9a-926a-4f29-8ee3-578a69f42315"]
pub struct GltfMesh {
    pub primitives: Vec<GltfPrimitive>,
}

#[derive(Debug, Clone)]
pub struct GltfPrimitive {
    index: usize,
    material: Option<usize>,
}
