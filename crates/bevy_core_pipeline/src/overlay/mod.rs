use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin};
use bevy_ecs::{
    entity::Entity,
    prelude::{With, Resource},
    system::{Commands, Local, Query, Res},
};
use bevy_input::{keyboard::KeyCode, Input};
use bevy_reflect::TypeUuid;
use bevy_render::{
    extract_component::ExtractComponentPlugin,
    prelude::Shader,
    render_graph::{RenderGraph, SlotInfo, SlotType},
    renderer::RenderQueue,
    RenderApp, RenderStage,
};

mod camera_overlay;
mod overlay_node;
mod pipeline;
use bevy_time::Time;
use bevy_utils::{Duration, Instant};
pub use camera_overlay::{CameraOverlay, CameraOverlayBundle};

use crate::overlay::{overlay_node::graph, pipeline::OverlayPipeline};

use self::overlay_node::DiagnosticOverlayBuffer;

pub const OVERLAY_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1236245567947772696);

#[derive(Resource)]
pub(crate) struct OverlayDiagnostics {
    avg_fps: f32,
}

// TODO: This fails because `Res<Diagnostics>` isn't found
fn extract_overlay_diagnostics(
    mut commands: Commands,
    diags: Res<Diagnostics>,
    time: Res<Time>,
    mut last_update: Local<Option<Instant>>,
) {
    if last_update.is_none() {
        *last_update = time
            .last_update()
            .and_then(|lu| lu.checked_sub(Duration::from_secs(1)));
    }

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

// TODO: This fails because `Res<Diagnostics>` isn't found
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
/// # use bevy_internal::DefaultPlugins;
/// # use bevy_app::App;
/// # use bevy_core_pipeline::overlay::OverlayPlugin;
/// fn main() {
///     App::new()
///         .add_plugins_with(DefaultPlugins, |group| group.disable::<OverlayPlugin>())
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
        // TODO: This fails e.g. for `examples/ui/text.rs` because it also
        //       adds FrameTimeDiagnosticsPlugin
        // if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
        //     app.add_plugin(FrameTimeDiagnosticsPlugin::default());
        // }

        load_internal_asset!(
            app,
            OVERLAY_SHADER_HANDLE,
            "overlay.wgsl",
            Shader::from_wgsl
        );

        let trigger = self.trigger;
        app.add_plugin(ExtractComponentPlugin::<CameraOverlay>::default())
            .add_system(
                move |mut commands: Commands,
                      keyboard_input: Res<Input<KeyCode>>,
                      query: Query<Entity, With<CameraOverlay>>| {
                    if (trigger.is_none()
                        && keyboard_input.pressed(KeyCode::LControl)
                        && keyboard_input.pressed(KeyCode::LShift)
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

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        render_app
            .init_resource::<OverlayPipeline>()
            .init_resource::<DiagnosticOverlayBuffer>()
            .add_system_to_stage(
                RenderStage::Extract,
                camera_overlay::extract_overlay_camera_phases,
            )
            .add_system_to_stage(RenderStage::Extract, extract_overlay_diagnostics)
            .add_system_to_stage(RenderStage::Prepare, prepare_overlay_diagnostics);

        let pass_node_overlay = overlay_node::OverlayNode::new(&mut render_app.world);
        let mut graph = render_app.world.resource_mut::<RenderGraph>();

        let mut overlay_graph = RenderGraph::default();
        overlay_graph.add_node(graph::NODE, pass_node_overlay);
        let input_node_id =
            overlay_graph.set_input(vec![SlotInfo::new(graph::NODE_INPUT, SlotType::Entity)]);
        overlay_graph
            .add_slot_edge(
                input_node_id,
                graph::NODE_INPUT,
                graph::NODE,
                graph::IN_VIEW,
            );
        graph.add_sub_graph(graph::NAME, overlay_graph);
    }
}
