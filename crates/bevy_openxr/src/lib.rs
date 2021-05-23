use bevy_app::{
    AppBuilder, CoreStage, Plugin, PluginGroupBuilder, ScheduleRunnerPlugin, ScheduleRunnerSettings,
};
use bevy_ecs::prelude::*;
use bevy_openxr_core::{
    event::{XRCameraTransformsUpdated, XRState, XRViewSurfaceCreated, XRViewsCreated},
    math::XRMatrixComputation,
    XRConfigurationState, XRDevice,
};
use bevy_render::{
    camera::Camera,
    camera::CameraProjection,
    render_graph::{base::node, RenderGraph, WindowTextureNode},
    RenderPlugin, RenderStage,
};

pub mod prelude {
    pub use crate::{HandPoseEvent, OpenXRPlugin, OpenXRSettings};

    pub use openxr::HandJointLocations;
}

use bevy_utils::tracing::warn;
use bevy_wgpu::{WgpuBackend, WgpuOptions};
use bevy_window::{CreateWindow, Window, WindowId, Windows};
use openxr::HandJointLocations;

mod error;
mod hand_tracking;
mod platform;
mod projection;
mod xr_swapchain_node;
use xr_swapchain_node::XRSwapchainNode;
mod xr_window_texture_node;
use xr_window_texture_node::XRWindowTextureNode;

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
            add_3d_camera: true,
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

        let mut wgpu_options = app
            .world_mut()
            .get_resource::<WgpuOptions>()
            .cloned()
            .unwrap_or_else(WgpuOptions::default);

        // force to Vulkan
        wgpu_options.backend = WgpuBackend::Vulkan;
        warn!("Set WgpuBackend to WgpuBackend::Vulkan (only one supported for OpenXR currently)");

        app
            // FIXME should handposeevent be conditional based on options
            .insert_resource(wgpu_options)
            .insert_resource(ScheduleRunnerSettings::run_loop(
                std::time::Duration::from_micros(0),
            ))
            .add_plugin(ScheduleRunnerPlugin::default())
            .add_event::<HandPoseEvent>()
            .add_system_to_stage(CoreStage::PostUpdate, openxr_camera_system.system());
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
    mut camera_transforms_updated: EventReader<XRCameraTransformsUpdated>,
) {
    // FIXME: remove
    for event in view_surface_created_events.iter() {
        for (_, mut camera_projection) in camera_query.iter_mut() {
            // this is actually unnecessary?
            camera_projection.update(event.width as f32, event.height as f32);
        }
    }

    // initialize projection matrices on view creation
    for event in views_created_events.iter() {
        for (mut camera, camera_projection) in camera_query.iter_mut() {
            camera.depth_calculation = camera_projection.depth_calculation();
            camera.projection_matrices = event
                .views
                .iter()
                .map(|view| camera_projection.get_projection_matrix_fov(&view.fov))
                .collect::<Vec<_>>();
        }
    }

    for event in camera_transforms_updated.iter() {
        for (mut camera, _) in camera_query.iter_mut() {
            camera.position_matrices = event
                .transforms
                .iter()
                .map(|transform| transform.compute_xr_matrix())
                .collect::<Vec<_>>();
        }
    }
}

pub struct OpenXRWgpuPlugin;

impl Plugin for OpenXRWgpuPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(add_xr_render_graph.system())
            .add_system(handle_create_window_events.system())
            .add_system_to_stage(
                RenderStage::Draw,
                pre_render_system.exclusive_system(), // FIXME there should maybe be some ImmediatelyBeforeRender system
            )
            .add_system_to_stage(
                RenderStage::PostRender,
                post_render_system.exclusive_system(), // FIXME there should maybe be some ImmediatelyAfterPost system
            );
    }
}

fn pre_render_system(
    mut xr_device: ResMut<XRDevice>,
    wgpu_handles: ResMut<bevy_wgpu::WgpuRendererHandles>,
    mut wgpu_render_state: ResMut<bevy_wgpu::WgpuRenderState>,
    mut xr_configuration_state: ResMut<XRConfigurationState>,
) {
    let (state, texture_views) = xr_device.prepare_update(&wgpu_handles.device);

    let should_render = if let XRState::Running = state {
        true
    } else {
        false
    };

    if let Some(texture_views) = texture_views {
        xr_configuration_state.texture_views = Some(texture_views);
    }

    if should_render {
        xr_configuration_state.next_swap_chain_index = xr_device
            .get_swapchain_mut()
            .unwrap()
            .get_next_swapchain_image_index();
    }

    wgpu_render_state.should_render = should_render;
}

fn post_render_system(mut xr_device: ResMut<XRDevice>) {
    xr_device.finalize_update();
}

fn add_xr_render_graph(mut graph: ResMut<RenderGraph>) {
    let main_depth_texture: &WindowTextureNode = graph.get_node(node::MAIN_DEPTH_TEXTURE).unwrap();
    let descriptor = *main_depth_texture.descriptor();

    graph
        .replace_node(
            node::MAIN_DEPTH_TEXTURE,
            XRWindowTextureNode::new(descriptor),
        )
        .unwrap();

    graph
        .replace_node(node::PRIMARY_SWAP_CHAIN, XRSwapchainNode::new())
        .unwrap();

    let main_sampled_color_attachment: &WindowTextureNode =
        graph.get_node(node::MAIN_SAMPLED_COLOR_ATTACHMENT).unwrap();

    let descriptor = *main_sampled_color_attachment.descriptor();

    graph
        .replace_node(
            node::MAIN_SAMPLED_COLOR_ATTACHMENT,
            XRWindowTextureNode::new(descriptor),
        )
        .unwrap();
}

fn handle_create_window_events(
    mut windows: ResMut<Windows>,
    mut create_window_events: EventReader<CreateWindow>,
    // mut window_created_events: EventWriter<WindowCreated>,
) {
    for _create_window_event in create_window_events.iter() {
        if let None = windows.get_primary() {
            windows.add(Window::new(
                WindowId::primary(),
                &Default::default(),
                896,
                1008,
                1.,
                None,
            ));
        }

        /*
        window_created_events.send(WindowCreated {
            id: create_window_event.id,
        });
         */
    }
}
