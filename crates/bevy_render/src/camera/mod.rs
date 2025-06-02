mod camera;
mod clear_color;
mod projection;
mod view;
mod visibility;

pub use camera::*;
pub use clear_color::*;
pub use projection::*;
pub use view::*;
pub use visibility::*;

use crate::{extract_component::ExtractComponentPlugin, extract_resource::ExtractResourcePlugin};
use bevy_app::{App, Plugin};

#[derive(Default)]
pub struct CameraPlugin;

//TODO: make sure all visibility systems get used

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Camera>()
            .register_type::<ClearColor>()
            .register_type::<Exposure>()
            .register_type::<TemporalJitter>()
            .register_type::<MipBias>()
            .init_resource::<ClearColor>()
            .add_plugins((
                CameraProjectionPlugin,
                ExtractResourcePlugin::<ClearColor>::default(),
                ExtractComponentPlugin::<CameraMainTextureUsages>::default(),
            ));

        /* TODO: replacement for CameraDriverNode
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(ExtractSchedule, extract_cameras);
            let camera_driver_node = CameraDriverNode::new(render_app.world_mut());
            let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
            render_graph.add_node(crate::graph::CameraDriverLabel, camera_driver_node);
        }
        */
    }

    fn finish(&self, app: &mut App) {}
}
