use std::sync::Mutex;

use crate::tonemapping::ViewTonemappingPipeline;

use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    frame_graph::FrameGraph,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{BindGroup, BufferId, TextureViewId},
    view::{ViewTarget, ViewUniformOffset},
};

use super::Tonemapping;

#[derive(Default)]
pub struct TonemappingNode {
    cached_bind_group: Mutex<Option<(BufferId, TextureViewId, TextureViewId, BindGroup)>>,
    last_tonemapping: Mutex<Option<Tonemapping>>,
}

impl ViewNode for TonemappingNode {
    type ViewQuery = (
        &'static ViewUniformOffset,
        &'static ViewTarget,
        &'static ViewTonemappingPipeline,
        &'static Tonemapping,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        _render_context: &mut FrameGraph,
        (_view_uniform_offset, _target, _view_tonemapping_pipeline, tonemapping): QueryItem<
            Self::ViewQuery,
        >,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        Ok(())
    }
}
