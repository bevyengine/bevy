use super::{
    FrameGraph, Pass, PassNode, RenderContext, ResourceRelease, ResourceRequese, TypeIndex,
};

#[derive(Default)]
pub struct DevicePass {
    pub pass: Option<Pass>,
    pub resource_release_array: Vec<ResourceRelease>,
    pub resource_request_array: Vec<ResourceRequese>,
    pub name: String,
}

impl DevicePass {
    pub fn request_resources(&self, render_context: &mut RenderContext) {
        for resource in self.resource_request_array.iter() {
            render_context.resource_table.request_resource(
                resource,
                &render_context.render_device,
                render_context.transient_resource_cache,
            );
        }
    }

    pub fn release_resources(&self, render_context: &mut RenderContext) {
        for handle in self.resource_release_array.iter() {
            render_context
                .resource_table
                .release_resource(handle, render_context.transient_resource_cache);
        }
    }

    pub fn execute(&self, render_context: &mut RenderContext) {
        self.request_resources(render_context);

        if let Some(pass) = &self.pass {
            pass.render(render_context);
        }
        self.release_resources(render_context);
    }

    pub fn extra(&mut self, graph: &mut FrameGraph, handle: TypeIndex<PassNode>) {
        let pass_node = graph.get_pass_node(&handle);

        let resource_request_array = pass_node
            .resource_request_array
            .iter()
            .map(|handle| graph.get_resource_node(handle).request())
            .collect();

        let resource_release_array = pass_node
            .resource_release_array
            .iter()
            .map(|handle| graph.get_resource_node(handle).release())
            .collect();

        let pass_node = graph.get_pass_node_mut(&handle);

        let pass = pass_node.pass.take();

        self.resource_request_array = resource_request_array;
        self.pass = pass;
        self.resource_release_array = resource_release_array;

        self.name = pass_node.name.clone();
    }
}
