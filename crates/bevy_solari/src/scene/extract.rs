use super::{RaytracingMesh3d, RaytracingSceneBindings};
use bevy_asset::{AssetEvent, AssetId, Assets};
use bevy_ecs::{
    lifecycle::RemovedComponents,
    message::MessageReader,
    query::{Added, Changed, Or, With},
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
};
use bevy_pbr::{MeshMaterial3d, PreviousGlobalTransform, StandardMaterial};
use bevy_platform::collections::HashMap;
use bevy_render::{sync_world::RenderEntity, Extract};
use bevy_transform::components::GlobalTransform;

/// Creates or removes components in the render world related to raytracing instances.
pub fn extract_raytracing_scene_structural(
    new_instances: Extract<
        Query<
            (
                RenderEntity,
                &RaytracingMesh3d,
                &MeshMaterial3d<StandardMaterial>,
                &GlobalTransform,
                Option<&PreviousGlobalTransform>,
            ),
            Added<RaytracingMesh3d>,
        >,
    >,
    mut removed_raytracing_meshes: Extract<RemovedComponents<RaytracingMesh3d>>,
    render_entities: Extract<Query<RenderEntity>>,
    mut commands: Commands,
) {
    // Process removed components before additions, that way it properly handles same-frame removal->insertion
    for main_entity in removed_raytracing_meshes.read() {
        if let Ok(render_entity) = render_entities.get(main_entity) {
            commands.entity(render_entity).remove::<RaytracingMesh3d>();
        }
    }

    for (render_entity, mesh, material, transform, previous_frame_transform) in &new_instances {
        commands.entity(render_entity).insert((
            mesh.clone(),
            material.clone(),
            *transform,
            previous_frame_transform
                .cloned()
                .unwrap_or(PreviousGlobalTransform(transform.affine())),
        ));
    }
}

/// Copies the transforms of moved raytracing instances from the main world
/// straight into their GPU buffers.
pub fn extract_raytracing_scene_transforms(
    main_instances: Extract<
        Query<
            (
                RenderEntity,
                &GlobalTransform,
                Option<&PreviousGlobalTransform>,
            ),
            (
                Or<(Changed<GlobalTransform>, Changed<PreviousGlobalTransform>)>,
                With<RaytracingMesh3d>,
            ),
        >,
    >,
    bindings: Res<RaytracingSceneBindings>,
) {
    main_instances
        .par_iter()
        .for_each(|(render_entity, transform, previous_frame_transform)| {
            let previous_frame_transform = previous_frame_transform
                .cloned()
                .unwrap_or(PreviousGlobalTransform(transform.affine()));

            bindings.move_instance(render_entity, transform, &previous_frame_transform);
        });
}

/// Updates the mesh and material of existing raytracing instances in the render world.
pub fn extract_raytracing_scene_meshes_and_materials(
    main_instances: Extract<
        Query<
            (
                RenderEntity,
                &RaytracingMesh3d,
                &MeshMaterial3d<StandardMaterial>,
            ),
            Or<(
                Changed<RaytracingMesh3d>,
                Changed<MeshMaterial3d<StandardMaterial>>,
            )>,
        >,
    >,
    mut render_instances: Query<(&mut RaytracingMesh3d, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    for (render_entity, new_mesh, new_material) in &main_instances {
        if let Ok((mut mesh, mut material)) = render_instances.get_mut(render_entity) {
            *mesh = new_mesh.clone();
            *material = new_material.clone();
        }
    }
}

/// The set of [`StandardMaterial`] in the scene, mirrored into the render world.
#[derive(Resource, Default)]
pub struct StandardMaterialAssets {
    materials: HashMap<AssetId<StandardMaterial>, StandardMaterial>,
    /// Materials added or modified this frame.
    pub changed: Vec<AssetId<StandardMaterial>>,
    /// Materials removed this frame.
    pub removed: Vec<AssetId<StandardMaterial>>,
}

impl StandardMaterialAssets {
    pub fn get(&self, id: &AssetId<StandardMaterial>) -> Option<&StandardMaterial> {
        self.materials.get(id)
    }
}

/// Keeps [`StandardMaterialAssets`] up to date in the render world.
pub fn extract_raytracing_material_assets(
    main_materials: Extract<Res<Assets<StandardMaterial>>>,
    mut render_materials: ResMut<StandardMaterialAssets>,
    mut events: Extract<MessageReader<AssetEvent<StandardMaterial>>>,
) {
    let render_materials = &mut *render_materials;

    render_materials.changed.clear();
    render_materials.removed.clear();

    for event in events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                if let Some(material) = main_materials.get(*id) {
                    render_materials.materials.insert(*id, material.clone());
                    render_materials.changed.push(*id);
                }
            }
            AssetEvent::Removed { id } => {
                render_materials.materials.remove(id);
                render_materials.removed.push(*id);
            }
            AssetEvent::Unused { .. } | AssetEvent::LoadedWithDependencies { .. } => {}
        }
    }
}
