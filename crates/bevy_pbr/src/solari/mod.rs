pub(crate) use self::global_illumination::SolariGlobalIlluminationViewResources;
pub use self::global_illumination::{
    SolariGlobalIlluminationNode, SolariGlobalIlluminationPlugin, SolariGlobalIlluminationSettings,
};
use self::scene::SolariScenePlugin;
use bevy_app::{App, Plugin};
use bevy_ecs::{
    system::Resource,
    world::{FromWorld, World},
};
use bevy_render::{
    render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
        TextureViewDescriptor,
    },
    renderer::{RenderDevice, RenderQueue},
    settings::WgpuFeatures,
    RenderApp,
};

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
            | WgpuFeatures::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
            | WgpuFeatures::PUSH_CONSTANTS;
        match app.world.get_resource::<RenderDevice>() {
            Some(render_device) if render_device.features().contains(required_features) => {}
            _ => return,
        }

        app.sub_app_mut(RenderApp)
            .init_resource::<SpatiotemporalBlueNoise>();

        app.insert_resource(SolariSupported)
            .add_plugins((SolariScenePlugin, SolariGlobalIlluminationPlugin));
    }

    fn build(&self, _: &mut App) {}
}

#[derive(Resource)]
pub struct SolariSupported;

#[derive(Resource)]
pub struct SolariEnabled;

#[derive(Resource)]
pub(crate) struct SpatiotemporalBlueNoise(TextureView);

impl FromWorld for SpatiotemporalBlueNoise {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();

        let texture = render_device.create_texture_with_data(
            render_queue,
            &(TextureDescriptor {
                label: Some("solari_spatiotemporal_blue_noise"),
                size: Extent3d {
                    width: 64,
                    height: 64,
                    depth_or_array_layers: 32,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rg8Unorm,
                usage: TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            }),
            include_bytes!("stbn.ktx2"),
        );

        Self(texture.create_view(&TextureViewDescriptor::default()))
    }
}
