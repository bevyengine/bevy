use std::{marker::PhantomData, sync::Arc};

use bevy_ecs::resource::Resource;

use super::{
    AnyFrameGraphResource, AnyFrameGraphResourceDescriptor, ResourceNode, TypeHandle,
    VirtualResource,
};

pub struct PassNode {}

pub trait ImportToFrameGraph
where
    Self: Sized + GraphResource,
{
    fn import(self: Arc<Self>) -> AnyFrameGraphResource;
}

pub trait GraphResource: 'static {
    type Descriptor: GraphResourceDescriptor;

    fn borrow_resource(res: &AnyFrameGraphResource) -> &Self;

    fn get_desc(&self) -> &Self::Descriptor;
}

pub trait GraphResourceDescriptor: 'static + Clone + Into<AnyFrameGraphResourceDescriptor> {
    type Resource: GraphResource;
}

pub trait TypeEquals {
    type Other;
    fn same(value: Self) -> Self::Other;
}

impl<T: Sized> TypeEquals for T {
    type Other = Self;
    fn same(value: Self) -> Self::Other {
        value
    }
}

#[derive(Resource)]
pub struct FrameGraph {
    resource_nodes: Vec<ResourceNode>,
    pass_nodes: Vec<PassNode>,
}

pub struct GraphResourceNodeHandle<ResourceType> {
    pub resource_node_handle: TypeHandle<ResourceNode>,
    pub version: u32,
    _marker: PhantomData<ResourceType>,
}

impl<ResourceType> GraphResourceNodeHandle<ResourceType> {
    pub fn new(resource_node_handle: TypeHandle<ResourceNode>, version: u32) -> Self {
        Self {
            resource_node_handle,
            version,
            _marker: PhantomData,
        }
    }
}

impl FrameGraph {
    pub fn get_pass_node_mut(&self, handle: &TypeHandle<PassNode>) -> &PassNode {
        &self.pass_nodes[handle.index]
    }

    pub fn get_pass_node(&self, handle: &TypeHandle<PassNode>) -> &PassNode {
        &self.pass_nodes[handle.index]
    }

    pub fn get_resource_node_mut(&self, handle: &TypeHandle<PassNode>) -> &PassNode {
        &self.pass_nodes[handle.index]
    }

    pub fn get_resource_node(&self, handle: &TypeHandle<PassNode>) -> &PassNode {
        &self.pass_nodes[handle.index]
    }

    pub fn import<ResourceType>(
        &mut self,
        name: &str,
        resource: Arc<ResourceType>,
    ) -> GraphResourceNodeHandle<ResourceType>
    where
        ResourceType: ImportToFrameGraph,
    {
        let resource_node_handle = TypeHandle::new(self.resource_nodes.len());
        let virtual_resource = VirtualResource::Imported(ImportToFrameGraph::import(resource));
        let resource_node = ResourceNode::new(name, resource_node_handle, virtual_resource);

        let version = resource_node.version();

        self.resource_nodes.push(resource_node);

        GraphResourceNodeHandle::new(resource_node_handle, version)
    }

    pub fn create<DescriptorType>(&mut self, name: &str, desc: DescriptorType) -> GraphResourceNodeHandle<DescriptorType::Resource>
    where
        DescriptorType: GraphResourceDescriptor
            + TypeEquals<
                Other = <<DescriptorType as GraphResourceDescriptor>::Resource as GraphResource>::Descriptor,
            >,
    {
        let resource_node_handle = TypeHandle::new(self.resource_nodes.len());
        let virtual_resource = VirtualResource::Setuped(desc.into());
        let resource_node = ResourceNode::new(name, resource_node_handle, virtual_resource);

        let version = resource_node.version();

        self.resource_nodes.push(resource_node);

        GraphResourceNodeHandle::new(resource_node_handle, version)
    }
}
