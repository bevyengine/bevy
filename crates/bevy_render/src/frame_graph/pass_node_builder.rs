use alloc::sync::Arc;

use super::{
    FrameGraph, GraphRawResourceNodeHandle, GraphResource, GraphResourceDescriptor,
    GraphResourceNodeHandle, ImportToFrameGraph, Pass, ResourceRead, ResourceRef, ResourceWrite,
    TypeEquals,
};

pub struct PassNodeBuilder<'a> {
    graph: &'a mut FrameGraph,
    name: String,
    writes: Vec<GraphRawResourceNodeHandle>,
    reads: Vec<GraphRawResourceNodeHandle>,
    pass: Option<Box<dyn Pass>>,
}

impl<'a> Drop for PassNodeBuilder<'a> {
    fn drop(&mut self) {
        let pass_node = self.graph.pass_node(&self.name);
        pass_node.writes = self.writes.clone();
        pass_node.reads = self.reads.clone();
        pass_node.pass = self.pass.take();
    }
}

impl<'a> PassNodeBuilder<'a> {
    pub fn import<ResourceType>(
        &mut self,
        name: &str,
        resource: Arc<ResourceType>,
    ) -> GraphResourceNodeHandle<ResourceType>
    where
        ResourceType: ImportToFrameGraph,
    {
        self.graph.import(name, resource)
    }

    pub fn create<DescriptorType>(&mut self, name: &str, desc: DescriptorType) -> GraphResourceNodeHandle<DescriptorType::Resource>
    where
        DescriptorType: GraphResourceDescriptor
            + TypeEquals<
                Other = <<DescriptorType as GraphResourceDescriptor>::Resource as GraphResource>::Descriptor,
            >,
    {
        self.graph.create(name, desc)
    }

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

    pub fn new(name: &str, graph: &'a mut FrameGraph) -> Self {
        Self {
            graph,
            name: name.to_string(),
            writes: vec![],
            reads: vec![],
            pass: None,
        }
    }
}
