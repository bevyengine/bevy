mod blas;
mod types;

pub use types::RaytracingMesh3d;

use bevy_app::{App, Plugin};
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_render::{
    mesh::{allocator::allocate_and_free_meshes, RenderMesh},
    render_asset::prepare_assets,
    renderer::RenderDevice,
    settings::WgpuFeatures,
    Render, RenderApp, RenderSet,
};
use blas::{manage_blas, BlasManager};
use tracing::warn;

pub struct SolariScenePlugin;

impl Plugin for SolariScenePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<RaytracingMesh3d>();
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        let render_device = render_app.world().resource::<RenderDevice>();
        let features = render_device.features();
        if !features.contains(WgpuFeatures::EXPERIMENTAL_RAY_TRACING_ACCELERATION_STRUCTURE) {
            warn!("SolariScenePlugin not loaded. GPU lacks support for required feature: EXPERIMENTAL_RAY_TRACING_ACCELERATION_STRUCTURE.");
            return;
        }

        render_app.init_resource::<BlasManager>().add_systems(
            Render,
            manage_blas
                .in_set(RenderSet::PrepareAssets)
                .before(prepare_assets::<RenderMesh>)
                .after(allocate_and_free_meshes),
        );
    }
}
