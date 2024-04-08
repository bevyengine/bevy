mod fsr_manager;
mod settings;
mod util;

pub use self::settings::{FsrBundle, FsrQualityMode, FsrSettings};

use self::fsr_manager::FsrManager;
use crate::{
    core_3d::{
        graph::{Core3d, Node3d},
        Camera3d,
    },
    prepass::{DepthPrepass, MotionVectorPrepass, ViewPrepassTextures},
};
use bevy_app::{App, Plugin};
use bevy_core::FrameCount;
use bevy_ecs::{
    entity::Entity,
    query::{QueryItem, With},
    schedule::IntoSystemConfigs,
    system::{Commands, Query, Res, ResMut},
    world::World,
};
use bevy_math::Vec4Swizzles;
use bevy_render::{
    camera::{Camera, MipBias, Projection, TemporalJitter},
    render_graph::{NodeRunError, RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner},
    renderer::{RenderContext, RenderDevice},
    view::{ExtractedView, Msaa, ViewTarget},
    ExtractSchedule, MainWorld, Render, RenderApp, RenderSet,
};

pub struct FsrPlugin;

impl Plugin for FsrPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Msaa::Off)
            .register_type::<FsrSettings>()
            .register_type::<FsrQualityMode>();
    }

    fn finish(&self, app: &mut App) {
        let Some(render_device) = app
            .world
            .get_resource::<RenderDevice>()
            .map(RenderDevice::clone)
        else {
            return;
        };

        app.sub_app_mut(RenderApp)
            .insert_resource(
                FsrManager::new(render_device).expect("Failed to initialize FsrPlugin"),
            )
            .add_systems(ExtractSchedule, extract_fsr_settings)
            .add_systems(Render, prepare_fsr.in_set(RenderSet::Prepare))
            .add_render_graph_node::<ViewNodeRunner<FsrNode>>(Core3d, Node3d::Taa)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EndMainPass,
                    Node3d::Taa,
                    Node3d::Bloom,
                    Node3d::Tonemapping,
                ),
            );
    }
}

fn extract_fsr_settings(mut commands: Commands, mut main_world: ResMut<MainWorld>) {
    let mut cameras_3d = main_world
        .query_filtered::<(Entity, &Camera, &Projection, &mut FsrSettings), (
            With<Camera3d>,
            With<TemporalJitter>,
            With<DepthPrepass>,
            With<MotionVectorPrepass>,
        )>();

    for (entity, camera, camera_projection, mut fsr_settings) in
        cameras_3d.iter_mut(&mut main_world)
    {
        if camera.is_active {
            if let Projection::Perspective(perspective_projection) = camera_projection {
                commands
                    .get_or_spawn(entity)
                    .insert((fsr_settings.clone(), perspective_projection.clone()));
                fsr_settings.reset = false;
            }
        }
    }
}

fn prepare_fsr(
    mut query: Query<(
        &FsrSettings,
        &ExtractedView,
        &mut TemporalJitter,
        &mut MipBias,
    )>,
    mut fsr_manager: ResMut<FsrManager>,
    frame_count: Res<FrameCount>,
) {
    for (fsr_settings, view, mut temporal_jitter, mut mip_bias) in &mut query {
        let upscaled_resolution = view.viewport.zw();
        let input_resolution =
            FsrManager::get_input_resolution(upscaled_resolution, fsr_settings.quality_mode);

        fsr_manager.recreate_context_if_needed(input_resolution, upscaled_resolution, view.hdr);

        // TODO: Set internal render resolution and upscale resolution

        *temporal_jitter =
            FsrManager::get_temporal_jitter(input_resolution, upscaled_resolution, *frame_count);

        mip_bias.0 = FsrManager::get_mip_bias(fsr_settings.quality_mode);
    }
}

#[derive(Default)]
pub struct FsrNode;

impl ViewNode for FsrNode {
    type ViewQuery = (
        &'static FsrSettings,
        &'static ViewTarget,
        &'static ViewPrepassTextures,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        (_fsr_settings, _view_target, _prepass_textures): QueryItem<Self::ViewQuery>,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        // TODO

        Ok(())
    }
}
