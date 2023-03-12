use bevy_app::{App, Plugin};
use bevy_render::{render_resource::WgpuFeatures, renderer::RenderDevice};

pub struct DebugOverlaysPlugin {}

impl Plugin for DebugOverlaysPlugin {
    fn build(&self, app: &mut App) {
        let wgpu_features = app.world.resource::<RenderDevice>().features();
        if !wgpu_features.contains(WgpuFeatures::TIMESTAMP_QUERY) {
            panic!("DebugOverlaysPlugin added but RenderPlugin::wgpu_settings did not contain WgpuFeatures::TIMESTAMP_QUERY.");
        }
    }
}
