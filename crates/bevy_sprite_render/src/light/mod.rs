mod ambient_light;
mod gpu;
mod point_light;

use bevy_app::{App, Plugin};
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_extract::extract_component::ExtractComponentPlugin;
use bevy_render::{
    extract_resource::ExtractResourcePlugin, GpuResourceAppExt, Render, RenderApp, RenderSystems,
};
use bevy_sprite_light::{GlobalAmbientLight2d, PointLight2d};
use gpu::{prepare_lights_2d_buffer, Lights2dBuffer};

/// Adds 2d lighting support.
#[derive(Default)]
pub struct Lighting2dPlugin;

impl Plugin for Lighting2dPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GlobalAmbientLight2d>().add_plugins((
            ExtractComponentPlugin::<PointLight2d, RenderApp, Self>::default(),
            ExtractResourcePlugin::<GlobalAmbientLight2d, Self>::default(),
        ));

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_gpu_resource::<Lights2dBuffer>()
                .add_systems(
                    Render,
                    prepare_lights_2d_buffer.in_set(RenderSystems::PrepareResources),
                );
        }
    }
}
