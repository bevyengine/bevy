use super::{
    FrameGraph, FrameGraphError, GraphRawResourceNodeHandle, GraphResourceNodeHandle,
    RenderContext, RenderPassInfo, ResourceNode, ResourceRead, ResourceRef, ResourceWrite,
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
    pub fn read<ResourceType>(
        &mut self,
        _graph: &FrameGraph,
        resource_node_handle: GraphResourceNodeHandle<ResourceType>,
    ) -> ResourceRef<ResourceType, ResourceRead> {
        let handle = resource_node_handle.raw();

        if !self.reads.contains(&handle) {
            self.reads.push(handle);
        }

        ResourceRef::new(resource_node_handle.handle)
    }

    pub fn write<ResourceType>(
        &mut self,
        graph: &mut FrameGraph,
        resource_node_handle: GraphResourceNodeHandle<ResourceType>,
    ) -> ResourceRef<ResourceType, ResourceWrite> {
        let resource_node = &mut graph.get_resource_node_mut(&resource_node_handle.handle);
        resource_node.new_version();

        let new_resource_node_handle = GraphRawResourceNodeHandle {
            handle: resource_node_handle.handle,
            version: resource_node.version(),
        };

        self.writes.push(new_resource_node_handle);

        ResourceRef::new(resource_node_handle.handle)
    }

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
