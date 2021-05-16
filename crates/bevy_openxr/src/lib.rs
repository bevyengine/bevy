use bevy_app::{
    AppBuilder, Plugin, PluginGroupBuilder, ScheduleRunnerPlugin, ScheduleRunnerSettings,
};
use bevy_ecs::prelude::*;
use bevy_openxr_core::{
    event::{XRViewSurfaceCreated, XRViewsCreated},
    XrFovf,
};
use bevy_render::{camera::Camera, camera::CameraProjection, RenderPlugin};

pub mod prelude {
    pub use crate::{HandPoseEvent, OpenXRPlugin, OpenXRSettings};

    pub use openxr::HandJointLocations;
}

use openxr::HandJointLocations;

mod error;
mod hand_tracking;
mod platform;
mod projection;

pub use hand_tracking::*;
pub use projection::*;

// FIXME: any better way for this? Works only for DefaultPlugins probably.
pub fn add_plugins_fn(group: &mut PluginGroupBuilder) -> &mut PluginGroupBuilder {
    group.add_before::<bevy_core::CorePlugin, _>(OpenXRPlugin);
    //group.remove::<bevy_winit::WinitPlugin>();
    group.remove::<RenderPlugin>();
    group.add_after::<bevy_scene::ScenePlugin, _>(get_render_plugin());

    group
}

pub fn get_render_plugin() -> RenderPlugin {
    RenderPlugin {
        base_render_graph_config: Some(bevy_render::render_graph::base::BaseRenderGraphConfig {
            add_2d_camera: false,
            add_3d_camera: false,
            add_xr_camera: true,
            ..Default::default()
        }),
    }
}

#[derive(Default)]
pub struct OpenXRPlugin;

#[derive(Debug)]
pub struct OpenXRSettings {}

impl Default for OpenXRSettings {
    fn default() -> Self {
        OpenXRSettings {}
    }
}

impl Plugin for OpenXRPlugin {
    fn build(&self, app: &mut AppBuilder) {
        {
            let settings = app.world_mut().insert_resource(OpenXRSettings::default());

            println!("Settings: {:?}", settings);
        };

        // must be initialized at startup, so that bevy_wgpu has access
        platform::initialize_openxr();

        app
            // FIXME should handposeevent be conditional based on options
            .insert_resource(ScheduleRunnerSettings::run_loop(
                std::time::Duration::from_micros(0),
            ))
            .add_plugin(ScheduleRunnerPlugin::default())
            .add_event::<HandPoseEvent>()
            .add_system(openxr_camera_system.system());
    }
}

pub struct HandPoseEvent {
    pub left: Option<HandJointLocations>,
    pub right: Option<HandJointLocations>,
}

impl std::fmt::Debug for HandPoseEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "(left: {}, right: {})",
            self.left.is_some(),
            self.right.is_some()
        )
    }
}

fn openxr_camera_system(
    mut camera_query: Query<(&mut Camera, &mut XRProjection)>,
    mut view_surface_created_events: EventReader<XRViewSurfaceCreated>,
    mut views_created_events: EventReader<XRViewsCreated>,
) {
    for event in view_surface_created_events.iter() {
        for (_, mut camera_projection) in camera_query.iter_mut() {
            // this is actually unnecessary?
            camera_projection.update(event.width as f32, event.height as f32);
        }
    }

    for event in views_created_events.iter() {
        for (mut camera, camera_projection) in camera_query.iter_mut() {
            camera.multiview_projection_matrices = event
                .views
                .iter()
                .map(|view| camera_projection.get_projection_matrix_fov(&view.fov))
                .collect::<Vec<_>>();

            camera.depth_calculation = camera_projection.depth_calculation();
        }
    }
}
