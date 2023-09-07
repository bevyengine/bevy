pub use self::global_illumination::{
    SolariGlobalIlluminationDebugView, SolariGlobalIlluminationNode,
    SolariGlobalIlluminationPlugin, SolariGlobalIlluminationSettings,
};
use self::scene::SolariScenePlugin;
use bevy_app::{App, Plugin};
use bevy_ecs::system::Resource;
use bevy_render::{renderer::RenderDevice, settings::WgpuFeatures};

mod global_illumination;
mod scene;

#[derive(Default)]
pub struct SolariPlugin;

impl Plugin for SolariPlugin {
    fn finish(&self, app: &mut App) {
        let required_features = WgpuFeatures::RAY_TRACING_ACCELERATION_STRUCTURE
            | WgpuFeatures::RAY_QUERY
            | WgpuFeatures::TEXTURE_BINDING_ARRAY
            | WgpuFeatures::BUFFER_BINDING_ARRAY
            | WgpuFeatures::STORAGE_RESOURCE_BINDING_ARRAY
            | WgpuFeatures::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
            | WgpuFeatures::PARTIALLY_BOUND_BINDING_ARRAY
            | WgpuFeatures::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES;
        match app.world.get_resource::<RenderDevice>() {
            Some(render_device) if render_device.features().contains(required_features) => {}
            _ => return,
        }

        app.insert_resource(SolariSupported)
            .add_plugins((SolariScenePlugin, SolariGlobalIlluminationPlugin));
    }

    fn build(&self, _: &mut App) {}
}

#[derive(Resource)]
pub struct SolariSupported;

#[derive(Resource)]
pub struct SolariEnabled;
