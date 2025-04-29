use std::sync::Mutex;

use crate::fxaa::{CameraFxaaPipeline, Fxaa};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    frame_graph::FrameGraph,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{BindGroup, TextureViewId},
    view::ViewTarget,
};

#[derive(Default)]
pub struct FxaaNode {
    cached_texture_bind_group: Mutex<Option<(TextureViewId, BindGroup)>>,
}

impl ViewNode for FxaaNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static CameraFxaaPipeline,
        &'static Fxaa,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        _frame_graph: &mut FrameGraph,
        (_target, _pipeline, _fxaa): QueryItem<Self::ViewQuery>,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        //todo

        Ok(())
    }
}
