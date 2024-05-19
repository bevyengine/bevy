#[allow(clippy::module_inception)]
mod camera;
mod camera_driver_node;
mod clear_color;
mod manual_texture_view;
mod projection;

pub use camera::*;
pub use camera_driver_node::*;
pub use clear_color::*;
pub use manual_texture_view::*;
pub use projection::*;

use crate::batching::gpu_preprocessing::GpuPreprocessingSupport;
use crate::{
    extract_component::ExtractComponentPlugin, extract_resource::ExtractResourcePlugin,
    render_graph::RenderGraph, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_app::prelude::*;
use bevy_ecs::schedule::IntoSystemConfigs;

#[derive(Default)]
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn setup(&self, app: &mut App) {
        app.register_type::<Camera>()
            .register_type::<ClearColor>()
            .register_type::<CameraRenderGraph>()
            .register_type::<CameraMainTextureUsages>()
            .register_type::<Exposure>()
            .register_type::<TemporalJitter>()
            .register_type::<MipBias>()
            .init_resource::<ManualTextureViews>()
            .init_resource::<ClearColor>()
            .add_plugins((
                CameraProjectionPlugin::<Projection>::default(),
                CameraProjectionPlugin::<OrthographicProjection>::default(),
                CameraProjectionPlugin::<PerspectiveProjection>::default(),
                ExtractResourcePlugin::<ManualTextureViews>::default(),
                ExtractResourcePlugin::<ClearColor>::default(),
                ExtractComponentPlugin::<CameraMainTextureUsages>::default(),
            ));
    }

    fn required_sub_apps(&self) -> Vec<InternedAppLabel> {
        vec![RenderApp.intern()]
    }

    fn ready_to_finalize(&self, app: &mut App) -> bool {
        let Some(render_app) = app.get_sub_app(RenderApp) else {
            return false;
        };
        render_app.world().contains_resource::<GpuPreprocessingSupport>()
    }

    fn finalize(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .init_resource::<SortedCameras>()
            .add_systems(ExtractSchedule, extract_cameras)
            .add_systems(Render, sort_cameras.in_set(RenderSet::ManageViews));
        let camera_driver_node = CameraDriverNode::new(render_app.world_mut());
        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(crate::graph::CameraDriverLabel, camera_driver_node);
    }
}
