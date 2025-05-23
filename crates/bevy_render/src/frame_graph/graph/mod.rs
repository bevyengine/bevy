mod graph_runner;

use std::borrow::Cow;

pub use graph_runner::*;

use alloc::sync::Arc;

use bevy_ecs::resource::Resource;

use crate::render_resource::BindGroupLayout;

use super::{
    BindGroupHandleBuilder, DevicePass, FrameGraphError, Handle,
    IntoArcTransientResource, PassBuilder, PassNode, PassNodeBuilder, RenderContext, ResourceBoard,
    ResourceNode, TransientResource, TransientResourceDescriptor, TypeEquals, TypeIndex,
    VirtualResource,
};

pub struct CompiledFrameGraph {
    device_passes: Vec<DevicePass>,
}

impl CompiledFrameGraph {
    pub fn execute(&self, render_context: &mut RenderContext) {
        for device_pass in self.device_passes.iter() {
            device_pass.execute(render_context);
        }
    }
}

#[derive(Resource, Default)]
pub struct FrameGraph {
    pub(crate) resource_nodes: Vec<ResourceNode>,
    pub(crate) pass_nodes: Vec<PassNode>,
    pub(crate) compiled_frame_graph: Option<CompiledFrameGraph>,
    pub(crate) resource_board: ResourceBoard,
}

impl FrameGraph {
    fn reset(&mut self) {
        self.pass_nodes = vec![];
        self.resource_nodes = vec![];
        self.compiled_frame_graph = None;
        self.resource_board = ResourceBoard::default();
    }

    pub fn execute(&mut self, render_context: &mut RenderContext) {
        if self.compiled_frame_graph.is_none() {
            return;
        }

        if let Some(compiled_frame_graph) = &mut self.compiled_frame_graph {
            compiled_frame_graph.execute(render_context);
        }

        self.reset();
    }

    pub fn compute_resource_lifetime(&mut self) {
        for pass_node in self.pass_nodes.iter_mut() {
            for resource_handle in pass_node.reads.iter() {
                let resource_node = &mut self.resource_nodes[resource_handle.index.index];
                resource_node.update_lifetime(pass_node.handle);
            }

            for resource_handle in pass_node.writes.iter() {
                let resource_node = &mut self.resource_nodes[resource_handle.index.index];
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
    pub fn put(&mut self, key: &str, handle: TypeIndex<ResourceNode>) {
        let key = key.into();
        self.resource_board.put(key, handle);
    }

    pub fn get<ResourceType: TransientResource>(
        &self,
        key: &str,
    ) -> Result<Handle<ResourceType>, FrameGraphError> {
        let key = key.into();

        self.resource_board
            .get(&key)
            .map(|handle| {
                let version = self.resource_nodes[handle.index].version();
                Handle::new(*handle, version)
            })
            .ok_or(FrameGraphError::ResourceBoardKey { key })
    }

    pub fn create_pass_node_bulder(&mut self, name: &str) -> PassNodeBuilder {
        PassNodeBuilder::new(name, self)
    }

    pub fn create_pass_builder(&mut self, name: &str) -> PassBuilder {
        PassBuilder::new(self.create_pass_node_bulder(name))
    }

    pub fn create_bind_group_handle_builder(
        &mut self,
        label: Option<Cow<'static, str>>,
        layout: &BindGroupLayout,
    ) -> BindGroupHandleBuilder {
        BindGroupHandleBuilder::new(label, layout.clone(), self)
    }

    pub fn pass_node(&mut self, name: &str) -> &mut PassNode {
        let handle = TypeIndex::new(self.pass_nodes.len());
        let pass_node = PassNode::new(name, handle);
        self.pass_nodes.push(pass_node);

        self.get_pass_node_mut(&handle)
    }

    pub fn get_pass_node_mut(&mut self, handle: &TypeIndex<PassNode>) -> &mut PassNode {
        &mut self.pass_nodes[handle.index]
    }

    pub fn get_pass_node(&self, handle: &TypeIndex<PassNode>) -> &PassNode {
        &self.pass_nodes[handle.index]
    }

    pub fn get_resource_node_mut(
        &mut self,
        handle: &TypeIndex<ResourceNode>,
    ) -> &mut ResourceNode {
        &mut self.resource_nodes[handle.index]
    }

    pub fn get_resource_node(&self, handle: &TypeIndex<ResourceNode>) -> &ResourceNode {
        &self.resource_nodes[handle.index]
    }

    pub fn import<ResourceType>(
        &mut self,
        name: &str,
        resource: Arc<ResourceType>,
    ) -> Handle<ResourceType>
    where
        ResourceType: IntoArcTransientResource,
    {
        let key = name.into();
        if let Some(raw_handle) = self.resource_board.get(&key) {
            let version = self.resource_nodes[raw_handle.index].version();
            return Handle::new(raw_handle.clone(), version);
        }

        let resource_node_handle = TypeIndex::new(self.resource_nodes.len());
        let virtual_resource = VirtualResource::Imported(
            IntoArcTransientResource::into_arc_transient_resource(resource),
        );
        let resource_node = ResourceNode::new(name, resource_node_handle, virtual_resource);

        let version = resource_node.version();

        self.resource_nodes.push(resource_node);

        let handle = Handle::new(resource_node_handle, version);
        self.put(name, handle.raw.index);

        handle
    }

    pub fn get_or_create<DescriptorType>(&mut self, name: &str, desc: DescriptorType) -> Handle<DescriptorType::Resource>
    where
        DescriptorType: TransientResourceDescriptor
            + TypeEquals<
                Other = <<DescriptorType as TransientResourceDescriptor>::Resource as TransientResource>::Descriptor,
            >,
    {
        let key = name.into();
        if let Some(raw_handle) = self.resource_board.get(&key) {
            let version = self.resource_nodes[raw_handle.index].version();

            return Handle::new(raw_handle.clone(), version);
        }

        let handle = self.create(name, desc);

        self.resource_board.put(key, handle.raw.index);

        handle
    }

    pub fn create<DescriptorType>(&mut self, name: &str, desc: DescriptorType) -> Handle<DescriptorType::Resource>
    where
        DescriptorType: TransientResourceDescriptor
            + TypeEquals<
                Other = <<DescriptorType as TransientResourceDescriptor>::Resource as TransientResource>::Descriptor,
            >,
    {
        let resource_node_handle = TypeIndex::new(self.resource_nodes.len());
        let virtual_resource = VirtualResource::Setuped(desc.into());
        let resource_node = ResourceNode::new(name, resource_node_handle, virtual_resource);

        let version = resource_node.version();

        self.resource_nodes.push(resource_node);

        Handle::new(resource_node_handle, version)
    }
}
