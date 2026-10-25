mod extract;
mod node;
mod prepare;

use crate::{scene::RaytracingSceneBindings, SolariPlugins};
use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::embedded_asset;
use bevy_camera::Hdr;
use bevy_core_pipeline::{
    schedule::{Core3d, Core3dSystems},
    tonemapping::tonemapping,
};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
};
use bevy_light::AtmosphereEnvironmentMapLight;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    init_gpu_resource, renderer::RenderDevice, ExtractSchedule, Render, RenderApp, RenderStartup,
    RenderSystems,
};
use extract::extract_pathtracer;
use node::{init_pathtracer_pipelines, pathtracer};
use prepare::prepare_pathtracer_accumulation_texture;
use tracing::warn;

/// Non-realtime pathtracing.
///
/// This plugin is meant to generate reference screenshots to compare against,
/// and is not intended to be used by games.
pub struct PathtracingPlugin;

impl Plugin for PathtracingPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "pathtracer.wesl");
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        let render_device = render_app.world().resource::<RenderDevice>();
        let features = render_device.features();
        if !features.contains(SolariPlugins::required_wgpu_features()) {
            warn!(
                "PathtracingPlugin not loaded. GPU lacks support for required features: {:?}.",
                SolariPlugins::required_wgpu_features().difference(features)
            );
            return;
        }

        render_app
            .add_systems(
                RenderStartup,
                init_pathtracer_pipelines.after(init_gpu_resource::<RaytracingSceneBindings>),
            )
            .add_systems(ExtractSchedule, extract_pathtracer)
            .add_systems(
                Render,
                prepare_pathtracer_accumulation_texture.in_set(RenderSystems::PrepareResources),
            )
            .add_systems(
                Core3d,
                pathtracer
                    .after(Core3dSystems::MainPass)
                    .before(tonemapping),
            );

        app.add_systems(PostUpdate, disable_atmosphere_env_map_filtering);
    }
}

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component, Default, Clone)]
#[require(Hdr)]
pub struct Pathtracer {
    pub reset: bool,
}

/// Turn off atmosphere cubemap filtering for pathtracer cameras to save performance, since the pathtracer does not require it.
fn disable_atmosphere_env_map_filtering(
    mut commands: Commands,
    lights: Query<(Entity, &AtmosphereEnvironmentMapLight), With<Pathtracer>>,
) {
    for (entity, light) in &lights {
        if !light.filtered {
            continue;
        }

        // Re-insert so the insert observer rebuilds the env map without filtering.
        commands
            .entity(entity)
            .insert(AtmosphereEnvironmentMapLight {
                filtered: false,
                ..light.clone()
            });
    }
}
