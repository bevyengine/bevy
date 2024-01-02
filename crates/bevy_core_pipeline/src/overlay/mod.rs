use bevy_app::{App, Plugin, Update};
use bevy_asset::{load_internal_asset, Handle};
use bevy_diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy_ecs::{
    change_detection::DetectChanges,
    entity::Entity,
    prelude::{Resource, With},
    schedule::IntoSystemConfigs,
    system::{Commands, Local, Query, Res},
};
use bevy_input::{keyboard::KeyCode, ButtonInput};
use bevy_render::{
    extract_component::ExtractComponentPlugin,
    prelude::Shader,
    render_graph::{RenderGraph, SlotInfo, SlotType},
    renderer::RenderQueue,
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};

mod camera_overlay;
mod overlay_node;
mod pipeline;
use bevy_time::{Real, Time};
use bevy_utils::{Duration, Instant};
pub use camera_overlay::{CameraOverlay, CameraOverlayBundle};

use crate::overlay::{overlay_node::graph, pipeline::OverlayPipeline};

use self::overlay_node::DiagnosticOverlayBuffer;

pub const OVERLAY_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(1236245567947772696);

#[derive(Resource)]
pub(crate) struct OverlayDiagnostics {
    avg_fps: f32,
}

fn extract_overlay_diagnostics(
    mut commands: Commands,
    diags: Extract<Res<DiagnosticsStore>>,
    time: Extract<Res<Time<Real>>>,
    mut last_update: Local<Option<Instant>>,
) {
    if last_update.is_none() {
        *last_update = time
            .last_update()
            .and_then(|lu| lu.checked_sub(Duration::from_secs(1)));
    }
    // todo: refactor - early returns
    if let Some(last_overlay_update) = *last_update {
        if let Some(current) = time.last_update() {
            if (current - last_overlay_update).as_secs_f32() > 0.15 {
                *last_update = time.last_update();
                commands.insert_resource(OverlayDiagnostics {
                    avg_fps: diags
                        .get(FrameTimeDiagnosticsPlugin::FPS)
                        .and_then(|diag| diag.average())
                        .unwrap_or_default() as f32,
                });
            }
        }
    }
}

fn prepare_overlay_diagnostics(
    buffer: Res<DiagnosticOverlayBuffer>,
    render_queue: Res<RenderQueue>,
    diagnostics: Res<OverlayDiagnostics>,
) {
    if diagnostics.is_changed() {
        buffer.write_buffer(diagnostics.as_ref(), render_queue.as_ref());
    }
}

/// Overlay to display the FPS. This plugin is part of the default plugins,
/// and enabled by default.
///
/// To disable it, you can either:
///
/// ## Remove it when adding the plugin group:
///
/// ```no_run
/// # use bevy_app::{App, NoopPluginGroup as DefaultPlugins, PluginGroup};
/// # use bevy_core_pipeline::overlay::OverlayPlugin;
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins.build().disable::<OverlayPlugin>())
///         .run();
/// }
/// ```
///
/// ## Disable the feature
///
/// Disable default features from Bevy, and do not enable the feature `overlay`
#[derive(Default)]
pub struct OverlayPlugin {
    /// [`KeyCode`] to use to trigger the overlay. If set to `None`, the default shortcut is used:
    /// Left-Control - Left-Shift - Tab
    pub trigger: Option<KeyCode>,
}

impl Plugin for OverlayPlugin {
    fn build(&self, app: &mut App) {
        if app
            .world
            .resource::<DiagnosticsStore>()
            .get(FrameTimeDiagnosticsPlugin::FPS)
            .is_none()
        {
            app.add_plugins(FrameTimeDiagnosticsPlugin);
        }

        load_internal_asset!(
            app,
            OVERLAY_SHADER_HANDLE,
            "overlay.wgsl",
            Shader::from_wgsl
        );

        let trigger = self.trigger;
        app.add_plugins(ExtractComponentPlugin::<CameraOverlay>::default())
            .add_systems(
                Update,
                move |mut commands: Commands,
                      keyboard_input: Res<ButtonInput<KeyCode>>,
                      query: Query<Entity, With<CameraOverlay>>| {
                    if (trigger.is_none()
                        && keyboard_input.pressed(KeyCode::ControlLeft)
                        && keyboard_input.pressed(KeyCode::ShiftLeft)
                        && keyboard_input.just_pressed(KeyCode::Tab))
                        || (trigger.is_some() && keyboard_input.just_pressed(trigger.unwrap()))
                    {
                        if let Ok(entity) = query.get_single() {
                            commands.entity(entity).despawn();
                        } else {
                            commands.spawn(CameraOverlayBundle::default());
                        }
                    }
                },
            );

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(
                ExtractSchedule,
                camera_overlay::extract_overlay_camera_phases,
            )
            .add_systems(ExtractSchedule, extract_overlay_diagnostics)
            .add_systems(
                Render,
                prepare_overlay_diagnostics.in_set(RenderSet::Prepare),
            );
    }
    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<OverlayPipeline>()
            .init_resource::<DiagnosticOverlayBuffer>();

        let pass_node_overlay = overlay_node::OverlayNode::new(&mut render_app.world);
        let mut graph = render_app.world.resource_mut::<RenderGraph>();

        let mut overlay_graph = RenderGraph::default();
        overlay_graph.add_node(graph::NODE, pass_node_overlay);
        let input_node_id =
            overlay_graph.set_input(vec![SlotInfo::new(graph::NODE_INPUT, SlotType::Entity)]);
        overlay_graph.add_slot_edge(
            input_node_id,
            graph::NODE_INPUT,
            graph::NODE,
            graph::IN_VIEW,
        );
        graph.add_sub_graph(graph::NAME, overlay_graph);
    }
}
