use super::{
    FrameGraph, FrameGraphError, Pass, PassNode, RenderContext, ResourceNode, TypeHandle,
    VirtualResource,
};

#[derive(Default)]
pub struct DevicePass {
    pub pass: Option<Box<dyn Pass>>,
    pub resource_release_array: Vec<TypeHandle<ResourceNode>>,
    pub resource_request_array: Vec<VirtualResource>,
    pub name: String,
}

impl DevicePass {
    pub fn execute(&self, render_context: &mut RenderContext) -> Result<(), FrameGraphError> {
        if let Some(pass) = &self.pass {
            pass.execute(render_context)?;
        }

        Ok(())
    }

    pub fn extra(&mut self, graph: &mut FrameGraph, handle: TypeHandle<PassNode>) {
        let pass_node = graph.get_pass_node(&handle);

        let resource_request_array = pass_node
            .resource_request_array
            .iter()
            .map(|handle| graph.get_resource_node(handle).resource.clone())
            .collect();

        let pass_node = graph.get_pass_node_mut(&handle);

        let pass = pass_node.pass.take();

        let resource_release_array = pass_node.resource_release_array.clone();

        self.resource_request_array = resource_request_array;
        self.pass = pass;
        self.resource_release_array = resource_release_array;

        self.name = pass_node.name.clone();
    }
}
