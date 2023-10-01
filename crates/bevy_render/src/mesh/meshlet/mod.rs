mod from_mesh;
mod gpu_scene;

pub use self::gpu_scene::MeshletGpuBuffers;
use crate::{renderer::RenderDevice, settings::WgpuFeatures, Render, RenderApp, RenderSet};
use bevy_app::{App, Plugin};
use bevy_asset::{Asset, AssetApp};
use bevy_ecs::{schedule::IntoSystemConfigs, system::Resource};
use bevy_math::Vec3;
use bevy_reflect::TypePath;
use serde::{Deserialize, Serialize};

pub struct MeshletPlugin;

impl Plugin for MeshletPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<MeshletMesh>();
    }

    fn finish(&self, app: &mut App) {
        let required_features = WgpuFeatures::MULTI_DRAW_INDIRECT;
        match app.world.get_resource::<RenderDevice>() {
            Some(render_device) if render_device.features().contains(required_features) => {}
            _ => return,
        }

        app.insert_resource(MeshletRenderingSupported);

        app.sub_app_mut(RenderApp)
            .insert_resource(MeshletRenderingSupported)
            .init_resource::<MeshletGpuBuffers>()
            .add_systems(
                Render,
                MeshletGpuBuffers::handle_meshlet_mesh_events.in_set(RenderSet::PrepareAssets),
            );
    }
}

#[derive(Resource)]
pub struct MeshletRenderingSupported;

#[derive(Asset, TypePath, Serialize, Deserialize)]
pub struct MeshletMesh {
    pub mesh_vertex_data: Box<[u8]>,
    pub meshlet_vertex_buffer: Box<[u32]>,
    pub meshlet_index_buffer: Box<[u8]>,
    pub meshlets: Box<[Meshlet]>,
    pub meshlet_bounding_spheres: Box<[MeshletBoundingSphere]>,
    pub meshlet_bounding_cones: Box<[MeshletBoundingCone]>,
}

#[derive(Serialize, Deserialize)]
pub struct Meshlet {
    pub meshlet_vertex_buffer_index: u32,
    pub meshlet_index_buffer_index: u32,
    pub meshlet_vertex_count: u32,
    pub meshlet_triangle_count: u32,
}

#[derive(Serialize, Deserialize)]
pub struct MeshletBoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

#[derive(Serialize, Deserialize)]
pub struct MeshletBoundingCone {
    pub apex: Vec3,
    pub axis: Vec3,
}
