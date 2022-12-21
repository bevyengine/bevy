#[allow(clippy::module_inception)]
mod camera;
mod camera_driver_node;
mod projection;

pub use camera::*;
pub use camera_driver_node::*;
pub use projection::*;

use crate::{render_graph::RenderGraph, RenderApp, RenderStage};
use bevy_app::{App, Plugin};

#[derive(Default)]
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Camera>()
            .register_type::<Viewport>()
            .register_type::<Option<Viewport>>()
            .register_type::<WindowOrigin>()
            .register_type::<ScalingMode>()
            .register_type::<CameraRenderGraph>()
            .register_type::<RenderTarget>()
            .add_plugin(CameraProjectionPlugin::<Projection>::default())
            .add_plugin(CameraProjectionPlugin::<OrthographicProjection>::default())
            .add_plugin(CameraProjectionPlugin::<PerspectiveProjection>::default());

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_system_to_stage(RenderStage::Extract, extract_cameras);

            let camera_driver_node = CameraDriverNode::new(&mut render_app.world);
            let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
            render_graph.add_node(crate::main_graph::node::CAMERA_DRIVER, camera_driver_node);
        }
    }
}
