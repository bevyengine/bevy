use super::{
    FrameGraphError, GraphRawResourceNodeHandle, RenderContext, RenderPassInfo, ResourceNode,
    TrackedRenderPass, TypeHandle,
};

pub struct PassNode {
    pub name: String,
    pub handle: TypeHandle<PassNode>,
    pub writes: Vec<GraphRawResourceNodeHandle>,
    pub reads: Vec<GraphRawResourceNodeHandle>,
    pub resource_request_array: Vec<TypeHandle<ResourceNode>>,
    pub resource_release_array: Vec<TypeHandle<ResourceNode>>,
    pub pass: Option<Box<dyn Pass>>,
}

impl PassNode {
    pub fn new(name: &str, handle: TypeHandle<PassNode>) -> Self {
        Self {
            name: name.to_string(),
            handle,
            writes: Default::default(),
            reads: Default::default(),
            resource_request_array: Default::default(),
            resource_release_array: Default::default(),
            pass: Some(Box::new(EmptyPass)),
        }
    }
}

pub trait Pass: 'static + Send + Sync {
    fn execute(&self, render_context: &mut RenderContext) -> Result<(), FrameGraphError>;
}

pub struct EmptyPass;

impl Pass for EmptyPass {
    fn execute(&self, _render_context: &mut RenderContext) -> Result<(), FrameGraphError> {
        Ok(())
    }
}

pub struct RenderPass {
    render_pass_info: RenderPassInfo,
    drawers: Vec<Box<dyn RenderPassDrawer>>,
}

pub trait RenderPassDrawer: 'static + Send + Sync {
    fn draw(&self, tracked_render_pass: &mut TrackedRenderPass) -> Result<(), FrameGraphError>;
}

impl Pass for RenderPass {
    fn execute(&self, render_context: &mut RenderContext) -> Result<(), FrameGraphError> {
        let mut tracked_render_pass = render_context.begin_render_pass(&self.render_pass_info)?;

        for drawer in self.drawers.iter() {
            drawer.draw(&mut tracked_render_pass)?;
        }

        tracked_render_pass.end();
        Ok(())
    }
}
