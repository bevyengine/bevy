use super::asset::MeshletMesh;
use bevy_asset::{AssetId, Assets, Handle};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::Without,
    system::{Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_render::{
    render_resource::{BindGroup, BindGroupLayout, Buffer},
    Extract,
};
use bevy_utils::HashMap;
use std::ops::Range;

pub fn extract_meshlet_meshes(
    mut main_world_commands: Extract<Commands>,
    mut render_world_commands: Commands,
    new_entities: Extract<Query<(Entity, &Handle<MeshletMesh>), Without<MeshletMeshGpuSceneSlice>>>,
    existing_entities: Extract<
        Query<(Entity, &MeshletMeshGpuSceneSlice), Without<Handle<MeshletMesh>>>,
    >,
    assets: Extract<Res<Assets<MeshletMesh>>>,
    mut gpu_scene: ResMut<MeshletGpuScene>,
) {
    for (entity, handle) in &new_entities {
        let scene_slice = gpu_scene.queue_meshlet_mesh_upload(handle, &assets);
        render_world_commands.entity(entity).insert(scene_slice);

        main_world_commands
            .entity(entity)
            .remove::<Handle<MeshletMesh>>();
    }

    for (entity, scene_slice) in &existing_entities {
        render_world_commands
            .entity(entity)
            .insert(scene_slice.clone());
    }
}

#[derive(Resource)]
pub struct MeshletGpuScene {
    vertex_data: Buffer,
    meshlet_vertices: Buffer,
    meshlet_indices: Buffer,
    meshlets: Buffer,
    meshlet_bounding_spheres: Buffer,
    meshlet_bounding_cone: Buffer,
    meshlet_mesh_slices: HashMap<AssetId<MeshletMesh>, MeshletMeshGpuSceneSlice>,
    bind_group_layout: BindGroupLayout,
}

impl FromWorld for MeshletGpuScene {
    fn from_world(world: &mut World) -> Self {
        Self {
            vertex_data: todo!(),
            meshlet_vertices: todo!(),
            meshlet_indices: todo!(),
            meshlets: todo!(),
            meshlet_bounding_spheres: todo!(),
            meshlet_bounding_cone: todo!(),
            meshlet_mesh_slices: todo!(),
            bind_group_layout: todo!(),
        }
    }
}

impl MeshletGpuScene {
    fn queue_meshlet_mesh_upload(
        &mut self,
        handle: &Handle<MeshletMesh>,
        assets: &Assets<MeshletMesh>,
    ) -> MeshletMeshGpuSceneSlice {
        todo!()
    }

    pub fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn create_per_frame_bind_group(&self) -> BindGroup {
        todo!()
    }
}

#[derive(Component, Clone)]
pub struct MeshletMeshGpuSceneSlice(Range<u64>);
