//! This modules contains functions that can make working with [`WorldAsset`] easier

use bevy_asset::{Assets, Handle};
use bevy_ecs::system::SystemState;
use bevy_mesh::{Mesh, Mesh3d};
use bevy_transform::helper::TransformHelper;
use bevy_world_serialization::WorldAsset;

/// Merge all the [`Mesh3d`] of a [`WorldAsset`] into a single [`Mesh`]
pub fn merge_all_mesh_3d(
    world_assets: &mut Assets<WorldAsset>,
    meshes: &mut Assets<Mesh>,
    scene_handle: &Handle<WorldAsset>,
) -> Option<Mesh> {
    let mut scene = world_assets.get_mut(scene_handle)?;
    let mut merged: Option<Mesh> = None;

    let mut system_state = SystemState::<TransformHelper>::new(&mut scene.world);
    let helper = system_state.get(&scene.world).ok()?;

    for entity_ref in scene.world.iter_entities() {
        let Some(mesh) = entity_ref
            .get::<Mesh3d>()
            .and_then(|mesh3d| meshes.get(mesh3d))
        else {
            continue;
        };
        let Ok(global_transform) = helper.compute_global_transform(entity_ref.id()) else {
            continue;
        };
        let transform = global_transform.compute_transform();
        let transformed = mesh.clone().transformed_by(transform);
        match &mut merged {
            Some(mesh) => {
                let _ = mesh.merge(&transformed);
            }
            None => {
                merged = Some(transformed);
            }
        }
    }
    merged
}
