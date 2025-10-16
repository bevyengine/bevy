use super::{extract::extract_render_task, node::RenderTaskNode, RenderTask};
use crate::{
    render_graph::{RenderGraphExt, ViewNodeRunner},
    renderer::RenderDevice,
    ExtractSchedule, RenderApp,
};
use bevy_app::{App, Plugin};
use std::marker::PhantomData;
use tracing::warn;

#[derive(Default)]
pub struct RenderTaskPlugin<T: RenderTask>(PhantomData<T>);

// TODO: Can't implement Plugin directly for T: RenderTask
impl<T: RenderTask> Plugin for RenderTaskPlugin<T> {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        // Get render app
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        let render_device = render_app.world().resource::<RenderDevice>();

        // Check features
        let features = render_device.features();
        if !features.contains(T::REQUIRED_FEATURES) {
            warn!(
                "{} not loaded. GPU lacks support for required features: {:?}.",
                std::any::type_name::<Self>(),
                T::REQUIRED_FEATURES.difference(features)
            );
            return;
        }

        // Check limits
        let mut should_exit = false;
        let fail_fn = |limit_name, required_limit_value, _| {
            warn!(
                "{} not loaded. GPU lacks support for required limits: {}={}.",
                std::any::type_name::<Self>(),
                limit_name,
                required_limit_value
            );
            should_exit = true;
        };
        T::REQUIRED_LIMITS.check_limits_with_fail_fn(&render_device.limits(), true, fail_fn);
        if should_exit {
            return;
        }

        // Setup app
        T::plugin_app_build(app);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        // Setup render app
        render_app
            .add_systems(ExtractSchedule, extract_render_task::<T>)
            .add_render_graph_node::<ViewNodeRunner<RenderTaskNode<T>>>(
                T::RenderNodeSubGraph::default(),
                T::RenderNodeLabel::default(),
            )
            .add_render_graph_edges_from_slice(
                T::RenderNodeSubGraph::default(),
                T::render_node_ordering(),
            );

        T::plugin_render_app_build(render_app);
    }
}
