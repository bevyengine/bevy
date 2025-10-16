use super::RenderTask;
use crate::{
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    renderer::RenderContext,
};
use bevy_ecs::{
    query::QueryItem,
    world::{FromWorld, World},
};
use std::marker::PhantomData;

#[derive(FromWorld)]
pub struct RenderTaskNode<T: RenderTask>(PhantomData<T>);

// TODO: Can't implement ViewNode directly for T: RenderTask
impl<T: RenderTask> ViewNode for RenderTaskNode<T> {
    type ViewQuery = ();

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        _: QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        todo!()
    }
}
