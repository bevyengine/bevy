#[allow(clippy::module_inception)]
mod camera;
mod camera_driver_node;
mod manual_texture_view;
mod projection;

pub use camera::*;
pub use camera_driver_node::*;
pub use manual_texture_view::*;
pub use projection::*;

use crate::{
    extract_resource::ExtractResourcePlugin, render_graph::RenderGraph, ExtractSchedule, Render,
    RenderApp, RenderSet,
};
use bevy_app::{App, Plugin};
use bevy_ecs::schedule::IntoSystemConfigs;

#[derive(Default)]
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Camera>()
            .register_type::<Viewport>()
            .register_type::<Option<Viewport>>()
            .register_type::<ScalingMode>()
            .register_type::<CameraRenderGraph>()
            .register_type::<RenderTarget>()
            .init_resource::<ManualTextureViews>()
            .add_plugins((
                CameraProjectionPlugin::<Projection>::default(),
                CameraProjectionPlugin::<OrthographicProjection>::default(),
                CameraProjectionPlugin::<PerspectiveProjection>::default(),
                ExtractResourcePlugin::<ManualTextureViews>::default(),
            ));

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<SortedCameras>()
                .add_systems(ExtractSchedule, extract_cameras)
                .add_systems(Render, sort_cameras.in_set(RenderSet::ManageViews));
            let camera_driver_node = CameraDriverNode::new(&mut render_app.world);
            let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
            render_graph.add_node(crate::main_graph::node::CAMERA_DRIVER, camera_driver_node);
        }
    }
}
