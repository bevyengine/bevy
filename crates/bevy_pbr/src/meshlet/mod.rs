mod asset;
mod from_mesh;
mod gpu_scene;

pub use self::asset::{Meshlet, MeshletBoundingCone, MeshletBoundingSphere, MeshletMesh};

use self::gpu_scene::{extract_meshlet_meshes, MeshletGpuScene};
use bevy_app::{App, Plugin};
use bevy_asset::AssetApp;
use bevy_ecs::system::Resource;
use bevy_render::{renderer::RenderDevice, settings::WgpuFeatures, ExtractSchedule, RenderApp};

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
            .add_systems(ExtractSchedule, extract_meshlet_meshes);
    }
}

#[derive(Resource)]
pub struct MeshletRenderingSupported;
