use alloc::sync::Arc;

use super::{
    FrameGraph, GraphRawResourceHandle, Handle, IntoArcTransientResource, Pass, PassBuilder,
    ResourceMaterial, ResourceRead, Ref, ResourceWrite, TransientResource,
    TransientResourceDescriptor, TypeEquals,
};

pub struct PassNodeBuilder<'a> {
    pub(crate) graph: &'a mut FrameGraph,
    pub(crate) name: String,
    writes: Vec<GraphRawResourceHandle>,
    reads: Vec<GraphRawResourceHandle>,
    pass: Option<Pass>,
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
    pub fn set_pass(&mut self, mut pass: Pass) {
        pass.label = Some(self.name.clone().into());
        self.pass = Some(pass)
    }

    pub fn create_pass_builder(self) -> PassBuilder<'a> {
        PassBuilder::new(self)
    }


    pub fn get_or_create<DescriptorType>(&mut self, name: &str, desc: DescriptorType) -> Handle<DescriptorType::Resource>
    where
        DescriptorType: TransientResourceDescriptor
            + TypeEquals<
                Other = <<DescriptorType as TransientResourceDescriptor>::Resource as TransientResource>::Descriptor,
            >,
    {
        self.graph.get_or_create(name, desc)
    }

    pub fn read_material<M: ResourceMaterial>(
        &mut self,
        material: &M,
    ) -> Ref<M::ResourceType, ResourceRead> {
        let handle = material.imported(self.graph);
        let read = self.read(handle);
        read
    }

    pub fn write_material<M: ResourceMaterial>(
        &mut self,
        material: &M,
    ) -> Ref<M::ResourceType, ResourceWrite> {
        let handle = material.imported(self.graph);
        let read = self.write(handle);
        read
    }

    pub fn import<ResourceType>(
        &mut self,
        name: &str,
        resource: Arc<ResourceType>,
    ) -> Handle<ResourceType>
    where
        ResourceType: IntoArcTransientResource,
    {
        self.graph.import(name, resource)
    }

    pub fn create<DescriptorType>(&mut self, name: &str, desc: DescriptorType) -> Handle<DescriptorType::Resource>
    where
        DescriptorType: TransientResourceDescriptor
            + TypeEquals<
                Other = <<DescriptorType as TransientResourceDescriptor>::Resource as TransientResource>::Descriptor,
            >,
    {
        self.graph.create(name, desc)
    }

    pub fn read<ResourceType: TransientResource>(
        &mut self,
        resource_handle: Handle<ResourceType>,
    ) -> Ref<ResourceType, ResourceRead> {
        let handle = resource_handle.raw.clone();

        if !self.reads.contains(&handle) {
            self.reads.push(handle.clone());
        }

        Ref::new(handle.index)
    }

    pub fn write<ResourceType: TransientResource>(
        &mut self,
        resource_handle: Handle<ResourceType>,
    ) -> Ref<ResourceType, ResourceWrite> {
        let index = resource_handle.raw.index.clone();

        let resource_node = &mut self
            .graph
            .get_resource_node_mut(&index);
        resource_node.new_version();

        let new_resource_node_handle = GraphRawResourceHandle {
            index: index,
            version: resource_node.version(),
        };

        self.writes.push(new_resource_node_handle);

        Ref::new(index)
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
