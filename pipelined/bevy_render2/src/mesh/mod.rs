#[allow(clippy::module_inception)]
mod mesh;
/// Generation for some primitive shape meshes.
pub mod shape;

use bevy_asset::AddAsset;
pub use mesh::*;

use bevy_app::{App, CoreStage, Plugin};
use bevy_ecs::system::IntoSystem;

pub struct MeshPlugin;

impl Plugin for MeshPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<Mesh>().add_system_to_stage(
            CoreStage::PostUpdate,
            mesh_resource_provider_system.system(),
        );
    }
}
