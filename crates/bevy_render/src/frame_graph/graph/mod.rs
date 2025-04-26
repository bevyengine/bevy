mod buffer;
mod graph_runner;
mod texture;

pub use graph_runner::*;

use alloc::sync::Arc;

use bevy_ecs::resource::Resource;

use super::{
    AnyFrameGraphResource, AnyFrameGraphResourceDescriptor, DevicePass, FrameGraphError,
    GraphResourceNodeHandle, ImportedResource, PassNode, PassNodeBuilder, RenderContext,
    ResourceNode, TypeHandle, VirtualResource,
};

pub trait ImportToFrameGraph
where
    Self: Sized + GraphResource,
{
    fn import(self: Arc<Self>) -> ImportedResource;
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

pub struct CompiledFrameGraph {
    device_passes: Vec<DevicePass>,
}

impl CompiledFrameGraph {
    pub fn execute(&self, render_context: &mut RenderContext) -> Result<(), FrameGraphError> {
        for device_pass in self.device_passes.iter() {
            device_pass.execute(render_context)?;
        }

        Ok(())
    }
}

#[derive(Resource, Default)]
pub struct FrameGraph {
    pub(crate) resource_nodes: Vec<ResourceNode>,
    pub(crate) pass_nodes: Vec<PassNode>,
    pub(crate) compiled_frame_graph: Option<CompiledFrameGraph>,
}

impl FrameGraph {
    fn reset(&mut self) {
        self.pass_nodes = vec![];
        self.resource_nodes = vec![];
        self.compiled_frame_graph = None;
    }

    pub fn execute(&mut self, render_context: &mut RenderContext) -> Result<(), FrameGraphError> {
        if self.compiled_frame_graph.is_none() {
            return Ok(());
        }

        if let Some(compiled_frame_graph) = &mut self.compiled_frame_graph {
            compiled_frame_graph.execute(render_context)?;
        }

        self.reset();

        Ok(())
    }

    pub fn compute_resource_lifetime(&mut self) {
        for pass_node in self.pass_nodes.iter_mut() {
            for resource_node_handle in pass_node.reads.iter() {
                let resource_node = &mut self.resource_nodes[resource_node_handle.handle.index];
                resource_node.update_lifetime(pass_node.handle);
            }

            for resource_node_handle in pass_node.writes.iter() {
                let resource_node = &mut self.resource_nodes[resource_node_handle.handle.index];
                resource_node.update_lifetime(pass_node.handle);
            }
        }

        for resource_index in 0..self.resource_nodes.len() {
            let resource_node = &self.resource_nodes[resource_index];

            if resource_node.first_use_pass.is_none() || resource_node.last_user_pass.is_none() {
                continue;
            }

            let first_pass_node_handle = resource_node.first_use_pass.unwrap();
            let first_pass_node = &mut self.pass_nodes[first_pass_node_handle.index];
            first_pass_node
                .resource_request_array
                .push(resource_node.handle);

            let last_pass_node_handle = resource_node.last_user_pass.unwrap();
            let last_pass_node = &mut self.pass_nodes[last_pass_node_handle.index];
            last_pass_node
                .resource_release_array
                .push(resource_node.handle);
        }
    }

    pub fn generate_compiled_frame_graph(&mut self) {
        if self.pass_nodes.is_empty() {
            return;
        }

        let mut device_passes = vec![];

        for index in 0..self.pass_nodes.len() {
            let handle = self.pass_nodes[index].handle;

            let mut device_pass = DevicePass::default();
            device_pass.extra(self, handle);

            device_passes.push(device_pass);
        }

        self.compiled_frame_graph = Some(CompiledFrameGraph { device_passes });
    }

    pub fn compile(&mut self) {
        if self.pass_nodes.is_empty() {
            return;
        }

        //todo cull

        self.compute_resource_lifetime();
        self.generate_compiled_frame_graph();
    }
}

impl FrameGraph {
    pub fn create_pass_node_bulder(&mut self, name: &str) -> PassNodeBuilder {
        PassNodeBuilder::new(name, self)
    }

    pub fn pass_node(&mut self, name: &str) -> &mut PassNode {
        let handle = TypeHandle::new(self.pass_nodes.len());
        let pass_node = PassNode::new(name, handle);
        self.pass_nodes.push(pass_node);

        self.get_pass_node_mut(&handle)
    }

    pub fn get_pass_node_mut(&mut self, handle: &TypeHandle<PassNode>) -> &mut PassNode {
        &mut self.pass_nodes[handle.index]
    }

    pub fn get_pass_node(&self, handle: &TypeHandle<PassNode>) -> &PassNode {
        &self.pass_nodes[handle.index]
    }

    pub fn get_resource_node_mut(
        &mut self,
        handle: &TypeHandle<ResourceNode>,
    ) -> &mut ResourceNode {
        &mut self.resource_nodes[handle.index]
    }

    pub fn get_resource_node(&self, handle: &TypeHandle<ResourceNode>) -> &ResourceNode {
        &self.resource_nodes[handle.index]
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
