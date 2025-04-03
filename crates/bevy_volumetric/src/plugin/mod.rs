use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, weak_handle, Assets, Handle};
use bevy_core_pipeline::core_3d::{
    graph::{Core3d, Node3d},
    prepare_core_3d_depth_textures,
};
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_math::{
    prelude::{Cuboid, Plane3d},
    Vec2, Vec3,
};
use bevy_render::{
    mesh::{Mesh, Meshable},
    render_graph::{RenderGraphApp, ViewNodeRunner},
    render_resource::{Shader, SpecializedRenderPipelines},
    sync_component::SyncComponentPlugin,
    ExtractSchedule, Render, RenderApp, RenderSet,
};
use systems::{
    extract_volumetric_fog, prepare_view_depth_textures_for_volumetric_fog,
    prepare_volumetric_fog_pipelines, prepare_volumetric_fog_uniforms,
};

use crate::{
    render::{
        NodeVolumetric, VolumetricFogNode, VolumetricFogPipeline, VolumetricFogUniformBuffer,
    },
    FogVolume, VolumetricFog,
};

mod systems;

/// The volumetric fog shader.
pub const VOLUMETRIC_FOG_HANDLE: Handle<Shader> =
    weak_handle!("481f474c-2024-44bb-8f79-f7c05ced95ea");

/// The plane mesh, which is used to render a fog volume that the camera is
/// inside.
///
/// This mesh is simply stretched to the size of the framebuffer, as when the
/// camera is inside a fog volume it's essentially a full-screen effect.
pub const PLANE_MESH: Handle<Mesh> = weak_handle!("92523617-c708-4fd0-b42f-ceb4300c930b");

/// The cube mesh, which is used to render a fog volume that the camera is
/// outside.
///
/// Note that only the front faces of this cuboid will be rasterized in
/// hardware. The back faces will be calculated in the shader via raytracing.
pub const CUBE_MESH: Handle<Mesh> = weak_handle!("4a1dd661-2d91-4377-a17a-a914e21e277e");

/// A plugin that implements volumetric fog.
#[derive(Default)]
pub struct VolumetricFogPlugin;

impl Plugin for VolumetricFogPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            VOLUMETRIC_FOG_HANDLE,
            "volumetric_fog.wgsl",
            Shader::from_wgsl
        );

        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        meshes.insert(&PLANE_MESH, Plane3d::new(Vec3::Z, Vec2::ONE).mesh().into());
        meshes.insert(&CUBE_MESH, Cuboid::new(1.0, 1.0, 1.0).mesh().into());

        app.register_type::<VolumetricFog>();

        app.add_plugins(SyncComponentPlugin::<FogVolume>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedRenderPipelines<VolumetricFogPipeline>>()
            .init_resource::<VolumetricFogUniformBuffer>()
            .add_systems(ExtractSchedule, extract_volumetric_fog)
            .add_systems(
                Render,
                (
                    prepare_volumetric_fog_pipelines.in_set(RenderSet::Prepare),
                    prepare_volumetric_fog_uniforms.in_set(RenderSet::Prepare),
                    prepare_view_depth_textures_for_volumetric_fog
                        .in_set(RenderSet::Prepare)
                        .before(prepare_core_3d_depth_textures),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<VolumetricFogPipeline>()
            .add_render_graph_node::<ViewNodeRunner<VolumetricFogNode>>(
                Core3d,
                NodeVolumetric::VolumetricFog,
            )
            .add_render_graph_edges(
                Core3d,
                // Volumetric fog is a postprocessing effect. Run it after the
                // main pass but before bloom.
                (
                    Node3d::EndMainPass,
                    NodeVolumetric::VolumetricFog,
                    Node3d::Bloom,
                ),
            );
    }
}
