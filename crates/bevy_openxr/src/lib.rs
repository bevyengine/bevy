use bevy_app::{
    AppBuilder, Events, Plugin, PluginGroupBuilder, ScheduleRunnerPlugin, ScheduleRunnerSettings,
};
use bevy_ecs::prelude::*;
use bevy_render::{camera::Camera, camera::CameraProjection};

pub mod prelude {
    pub use crate::{HandPoseEvent, OpenXRPlugin, OpenXRSettings};

    pub use openxr::HandJointLocations;
}

mod platform;
use openxr::HandJointLocations;
use platform::OpenXRInstance;

mod projection;
pub use projection::*;

mod error;
pub use error::Error;

mod hand_tracking;
pub use hand_tracking::*;

// FIXME: any better way for this? Works only for DefaultPlugins probably.
pub fn add_plugins_fn(group: &mut PluginGroupBuilder) -> &mut PluginGroupBuilder {
    group.add_before::<bevy_core::CorePlugin, _>(OpenXRPlugin);
    //group.remove::<bevy_winit::WinitPlugin>();
    group.remove::<bevy_render::RenderPlugin>();
    group.add_after::<bevy_scene::ScenePlugin, _>(bevy_render::RenderPlugin {
        base_render_graph_config: Some(bevy_render::render_graph::base::BaseRenderGraphConfig {
            add_2d_camera: false,
            add_3d_camera: false,
            add_xr_camera: true,
            ..Default::default()
        }),
    });

    group
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

#[derive(Default)]
pub struct XRState {
    #[allow(dead_code)]
    entry: Option<openxr::Entry>,
    #[allow(dead_code)]
    instance: Option<openxr::Instance>,
}

impl Plugin for OpenXRPlugin {
    fn build(&self, app: &mut AppBuilder) {
        {
            let settings = app.world_mut().insert_resource(OpenXRSettings::default());

            println!("Settings: {:?}", settings);
        };

        // must be initialized at startup, so that bevy_wgpu has access
        initialize_openxr();

        app.init_resource::<XRState>()
            .init_resource::<ProjectionState>()
            // FIXME should handposeevent be conditional based on options
            .insert_resource(ScheduleRunnerSettings::run_loop(
                std::time::Duration::from_micros(0),
            ))
            .add_plugin(ScheduleRunnerPlugin::default())
            .add_event::<HandPoseEvent>()
            //.add_startup_system(openxr_startup_system.system())
            .add_system(render_openxr_system.system())
            .add_system(openxr_camera_system.system());
    }
}

fn initialize_openxr() {
    //println!("Initialize openxr");
    //let start = Instant::now();
    let mut entry = openxr::Entry::load_bevy_openxr().unwrap();
    //println!("\tloading took {:?}", start.elapsed());

    let extensions = entry.enumerate_extensions().unwrap();
    //println!("Extensions: {:#?}", extensions);

    //let layers = entry.enumerate_layers().unwrap();
    //println!("Supported layers: {:?}", layers);

    let instance = entry.instantiate(&extensions).unwrap();

    //let instance_props = instance.properties().unwrap();
    //println!("loaded instance: {:?}", instance_props);

    //println!("\telapsed since initialize {:?}", start.elapsed());

    //state.entry = Some(entry);
    //state.instance = Some(instance);

    let wgpu_openxr = wgpu::wgpu_openxr::new(
        wgpu::BackendBit::VULKAN,
        &instance,
        wgpu::wgpu_openxr::OpenXROptions::default(),
    )
    .unwrap();

    bevy_openxr_core::set_openxr(wgpu_openxr, instance);

    //println!("\tfull init took {:?}", start.elapsed());
}

fn render_openxr_system(_state: ResMut<XRState>, _hand_pose_events: ResMut<Events<HandPoseEvent>>) {
    //render::render_once(&mut state, hand_pose_events);
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

#[derive(Default)]
struct ProjectionState {
    is_configured: bool,
}

fn openxr_camera_system(
    mut projection_state: ResMut<ProjectionState>,
    mut queries: QuerySet<(
        Query<(Entity, &mut Camera, &mut XRProjection)>,
        Query<Entity, Added<Camera>>,
    )>,
) {
    // FIXME ugly hack. handle resolution changes
    if projection_state.is_configured {
        return;
    }

    for (_entity, mut camera, mut camera_projection) in queries.q0_mut().iter_mut() {
        // FIXME handle xr events only
        camera_projection.update(1440., 1584.);

        camera.multiview_projection_matrices = vec![
            camera_projection.get_projection_matrix_fov(XrFovf {
                angle_left: -0.8552113,
                angle_right: 0.7853982,
                angle_up: 0.83775806,
                angle_down: -0.87266463,
            }),
            camera_projection.get_projection_matrix_fov(XrFovf {
                angle_left: -0.7853982,
                angle_right: 0.8552113,
                angle_up: 0.83775806,
                angle_down: -0.87266463,
            }),
        ];

        println!(
            "Updated projection matrices TO: {:#?}",
            camera.multiview_projection_matrices
        );

        //camera.projection_matrix = camera.multiview_projection_matrices[0]; // camera_projection.get_projection_matrix();
        camera.depth_calculation = camera_projection.depth_calculation();
    }

    projection_state.is_configured = true;
}
