//! Volumetric fog and volumetric lighting, also known as light shafts or god
//! rays.
//!
//! This module implements a more physically-accurate, but slower, form of fog
//! than the [`crate::fog`] module does. Notably, this *volumetric fog* allows
//! for light beams from directional lights to shine through, creating what is
//! known as *light shafts* or *god rays*.
//!
//! To add volumetric fog to a scene, add [`bevy_light::VolumetricFog`] to the
//! camera, and add [`bevy_light::VolumetricLight`] to directional lights that you wish to
//! be volumetric. [`bevy_light::VolumetricFog`] feature numerous settings that
//! allow you to define the accuracy of the simulation, as well as the look of
//! the fog. Currently, only interaction with directional lights that have
//! shadow maps is supported. Note that the overhead of the effect scales
//! directly with the number of directional lights in use, so apply
//! [`bevy_light::VolumetricLight`] sparingly for the best results.
//!
//! The overall algorithm, which is implemented as a postprocessing effect, is a
//! combination of the techniques described in [Scratchapixel] and [this blog
//! post]. It uses raymarching in screen space, transformed into shadow map
//! space for sampling and combined with physically-based modeling of absorption
//! and scattering. Bevy employs the widely-used [Henyey-Greenstein phase
//! function] to model asymmetry; this essentially allows light shafts to fade
//! into and out of existence as the user views them.
//!
//! [Scratchapixel]: https://www.scratchapixel.com/lessons/3d-basic-rendering/volume-rendering-for-developers/intro-volume-rendering.html
//!
//! [this blog post]: https://www.alexandre-pestana.com/volumetric-lights/
//!
//! [Henyey-Greenstein phase function]: https://www.pbr-book.org/4ed/Volume_Scattering/Phase_Functions#TheHenyeyndashGreensteinPhaseFunction

use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, Assets, Handle};
use bevy_core_pipeline::core_3d::{
    graph::{Core3d, Node3d},
    prepare_core_3d_depth_textures,
};
use bevy_ecs::{resource::Resource, schedule::IntoScheduleConfigs as _};
use bevy_light::FogVolume;
use bevy_math::{
    primitives::{Cuboid, Plane3d},
    Vec2, Vec3,
};
use bevy_mesh::{Mesh, Meshable};
use bevy_render::{
    render_graph::{RenderGraphExt, ViewNodeRunner},
    render_resource::SpecializedRenderPipelines,
    sync_component::SyncComponentPlugin,
    ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
};
use render::{VolumetricFogNode, VolumetricFogPipeline, VolumetricFogUniformBuffer};

use crate::{graph::NodePbr, volumetric_fog::render::init_volumetric_fog_pipeline};

pub mod render;

/// A plugin that implements volumetric fog.
pub struct VolumetricFogPlugin;

#[derive(Resource)]
pub struct FogAssets {
    plane_mesh: Handle<Mesh>,
    cube_mesh: Handle<Mesh>,
}

impl Plugin for VolumetricFogPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "volumetric_fog.wgsl");

        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        let plane_mesh = meshes.add(Plane3d::new(Vec3::Z, Vec2::ONE).mesh());
        let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0).mesh());

        app.add_plugins(SyncComponentPlugin::<FogVolume>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .insert_resource(FogAssets {
                plane_mesh,
                cube_mesh,
            })
            .init_resource::<SpecializedRenderPipelines<VolumetricFogPipeline>>()
            .init_resource::<VolumetricFogUniformBuffer>()
            .add_systems(RenderStartup, init_volumetric_fog_pipeline)
            .add_systems(ExtractSchedule, render::extract_volumetric_fog)
            .add_systems(
                Render,
                (
                    render::prepare_volumetric_fog_pipelines.in_set(RenderSystems::Prepare),
                    render::prepare_volumetric_fog_uniforms.in_set(RenderSystems::Prepare),
                    render::prepare_view_depth_textures_for_volumetric_fog
                        .in_set(RenderSystems::Prepare)
                        .before(prepare_core_3d_depth_textures),
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<VolumetricFogNode>>(
                Core3d,
                NodePbr::VolumetricFog,
            )
            .add_render_graph_edges(
                Core3d,
                // Volumetric fog is a postprocessing effect. Run it after the
                // main pass but before bloom.
                (Node3d::EndMainPass, NodePbr::VolumetricFog, Node3d::Bloom),
            );
    }
}
