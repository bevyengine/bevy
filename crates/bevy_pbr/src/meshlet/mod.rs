mod asset;
mod from_mesh;
mod gpu_scene;
mod persistent_buffer;
mod psb_wrappers;

pub use self::asset::{Meshlet, MeshletBoundingCone, MeshletBoundingSphere, MeshletMesh};

use self::gpu_scene::{
    extract_meshlet_meshes, perform_pending_meshlet_mesh_writes, MeshletGpuScene,
};
use bevy_app::{App, Plugin};
use bevy_asset::AssetApp;
use bevy_ecs::{schedule::IntoSystemConfigs, system::Resource};
use bevy_render::{
    renderer::RenderDevice, settings::WgpuFeatures, ExtractSchedule, Render, RenderApp, RenderSet,
};

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
            .init_resource::<MeshletGpuScene>()
            .add_systems(ExtractSchedule, extract_meshlet_meshes)
            .add_systems(
                Render,
                perform_pending_meshlet_mesh_writes.in_set(RenderSet::PrepareAssets),
            );
    }
}

#[derive(Resource)]
pub struct MeshletRenderingSupported;
