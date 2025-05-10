use std::borrow::Cow;

use alloc::sync::Arc;

use crate::render_resource::{BindGroupLayout, Buffer, Texture};

use super::{
    BindGroupDrawingBuilder, FrameGraph, FrameGraphBuffer, FrameGraphError, FrameGraphTexture,
    GraphRawResourceNodeHandle, GraphResource, GraphResourceDescriptor, GraphResourceNodeHandle,
    ImportToFrameGraph, Pass, PassTrait, ResourceBoardKey, ResourceMaterial, ResourceRead,
    ResourceRef, ResourceWrite, TypeEquals,
};

pub struct PassNodeBuilder<'a> {
    graph: &'a mut FrameGraph,
    name: String,
    writes: Vec<GraphRawResourceNodeHandle>,
    reads: Vec<GraphRawResourceNodeHandle>,
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
    pub fn set_pass<T: PassTrait>(&mut self, mut pass: T) {
        self.pass = Some(Pass::new(pass))
    }

    pub fn create_bind_group_drawing_builder<'b>(
        &'b mut self,
        label: Option<Cow<'static, str>>,
        layout: BindGroupLayout,
    ) -> BindGroupDrawingBuilder<'a, 'b> {
        BindGroupDrawingBuilder::new(label, layout, self)
    }

    pub fn read_from_board<ResourceType: GraphResource, Key: Into<ResourceBoardKey>>(
        &mut self,
        key: Key,
    ) -> Result<ResourceRef<ResourceType, ResourceRead>, FrameGraphError> {
        let key: ResourceBoardKey = key.into();
        let handle = self.graph.get(&key)?;
        let read = self.read(handle);
        Ok(read)
    }

    pub fn write_from_board<ResourceType: GraphResource, Key: Into<ResourceBoardKey>>(
        &mut self,
        key: Key,
    ) -> Result<ResourceRef<ResourceType, ResourceWrite>, FrameGraphError> {
        let key: ResourceBoardKey = key.into();
        let handle = self.graph.get(&key)?;
        let write = self.write(handle);
        Ok(write)
    }

    pub fn import_and_read_buffer(
        &mut self,
        buffer: &Buffer,
    ) -> ResourceRef<FrameGraphBuffer, ResourceRead> {
        let handle = buffer.make_resource_handle(self.graph);
        let read = self.read(handle);
        read
    }

    pub fn import_and_read_texture(
        &mut self,
        texture: &Texture,
    ) -> ResourceRef<FrameGraphTexture, ResourceRead> {
        let handle = texture.make_resource_handle(self.graph);
        let read = self.read(handle);
        read
    }

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
        resource_node_handle: GraphResourceNodeHandle<ResourceType>,
    ) -> ResourceRef<ResourceType, ResourceWrite> {
        let resource_node = &mut self
            .graph
            .get_resource_node_mut(&resource_node_handle.handle);
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
