use std::{borrow::Borrow, hash::Hash, ops::Deref};

use bevy_ecs::world::World;
use bevy_utils::HashMap;
use wgpu::{BindGroupEntry, BindingResource, BufferAddress, BufferBinding, BufferSize, Label};

use crate::{
    render_graph_v2::{NodeContext, RenderGraph, RenderGraphBuilder},
    render_resource::{
        BindGroup, BindGroupLayout, BindGroupLayoutEntry, Buffer, Sampler, TextureView,
    },
    renderer::RenderDevice,
};

use super::{
    ref_eq::RefEq, DescribedRenderResource, FromDescriptorRenderResource, IntoRenderResource,
    RenderDependencies, RenderHandle, RenderResource, RenderResourceId, ResourceTracker,
    ResourceType,
};

impl RenderResource for BindGroupLayout {
    const RESOURCE_TYPE: ResourceType = ResourceType::BindGroupLayout;

    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.new_bind_group_layout_direct(None, resource)
    }

    fn get_from_store<'a>(
        context: &'a NodeContext<'a>,
        resource: RenderHandle<'a, Self>,
    ) -> Option<&'a Self> {
        context.get_bind_group_layout(resource)
    }
}

impl DescribedRenderResource for BindGroupLayout {
    type Descriptor = Vec<BindGroupLayoutEntry>;

    fn new_with_descriptor<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        descriptor: Self::Descriptor,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.new_bind_group_layout_direct(Some(descriptor), resource)
    }

    fn get_descriptor<'a, 'g: 'a>(
        graph: &'a RenderGraphBuilder<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a Self::Descriptor> {
        graph.get_bind_group_layout_descriptor(resource)
    }
}

impl FromDescriptorRenderResource for BindGroupLayout {
    fn new_from_descriptor<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        descriptor: Self::Descriptor,
    ) -> RenderHandle<'g, Self> {
        graph.new_bind_group_layout_descriptor(descriptor)
    }
}

impl<'g> IntoRenderResource<'g> for &[BindGroupLayoutEntry] {
    type Resource = BindGroupLayout;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_bind_group_layout_descriptor(Vec::from(self))
    }
}

#[derive(Default)]
pub struct RenderGraphBindGroups<'g> {
    bind_groups: HashMap<RenderResourceId, RenderGraphBindGroupMeta<'g>>,
    existing_bind_groups: HashMap<RefEq<'g, BindGroup>, RenderResourceId>,
    queued_bind_groups: HashMap<RenderResourceId, RenderGraphBindGroupDescriptor<'g>>,
}

struct RenderGraphBindGroupMeta<'g> {
    layout: Option<RenderHandle<'g, BindGroupLayout>>,
    bind_group: RefEq<'g, BindGroup>,
}

impl<'g> RenderGraphBindGroups<'g> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_direct(
        &mut self,
        tracker: &mut ResourceTracker<'g>,
        mut dependencies: RenderDependencies<'g>,
        layout: Option<RenderHandle<'g, BindGroupLayout>>,
        bind_group: RefEq<'g, BindGroup>,
    ) -> RenderResourceId {
        self.existing_bind_groups
            .get(&bind_group)
            .copied()
            .unwrap_or_else(|| {
                if let Some(layout) = layout {
                    dependencies.read(layout);
                }
                let id = tracker.new_resource(ResourceType::BindGroup, Some(dependencies));
                self.existing_bind_groups.insert(bind_group.clone(), id);
                self.bind_groups
                    .insert(id, RenderGraphBindGroupMeta { layout, bind_group });
                id
            })
    }

    pub fn new_from_descriptor(
        &mut self,
        tracker: &mut ResourceTracker<'g>,
        descriptor: RenderGraphBindGroupDescriptor<'g>,
    ) -> RenderResourceId {
        let mut dependencies = descriptor.dependencies.clone();
        dependencies.read(descriptor.layout);
        let id = tracker.new_resource(ResourceType::BindGroup, Some(dependencies));
        self.queued_bind_groups.insert(id, descriptor);
        id
    }

    pub fn create_queued_bind_groups(
        &mut self,
        graph: &RenderGraph,
        world: &World,
        render_device: &RenderDevice,
        // view_entity: EntityRef<'g>,
    ) {
        let mut bind_group_cache = HashMap::new();
        for (
            id,
            RenderGraphBindGroupDescriptor {
                label,
                layout,
                dependencies,
                bindings,
            },
        ) in self.queued_bind_groups.drain()
        {
            let context = NodeContext {
                graph,
                world,
                dependencies,
                // entity: view_entity,
            };
            let raw_layout = context.get(layout);
            let bind_group = bind_group_cache
                .entry(bindings)
                .or_insert_with_key(|bindings| {
                    //awful rust lifetimes hack
                    let owned_bindings = bindings
                        .iter()
                        .map(|binding| binding.as_owned_binding(&context))
                        .collect::<Vec<_>>();
                    let raw_bindings = owned_bindings
                        .iter()
                        .map(|binding| binding.as_binding())
                        .collect::<Vec<_>>();
                    render_device.create_bind_group(label, raw_layout, &raw_bindings)
                });
            self.bind_groups.insert(
                id,
                RenderGraphBindGroupMeta {
                    layout: Some(layout),
                    bind_group: RefEq::Owned(bind_group.clone()),
                },
            );
        }
    }

    pub fn get(&self, id: RenderResourceId) -> Option<&BindGroup> {
        self.bind_groups
            .get(&id)
            .map(|meta| meta.bind_group.borrow())
    }

    pub fn get_layout(&self, id: RenderResourceId) -> Option<RenderHandle<'g, BindGroupLayout>> {
        let check_normal = self.bind_groups.get(&id).and_then(|meta| meta.layout);
        let check_queued = self.queued_bind_groups.get(&id).map(|queued| queued.layout);
        check_normal.or(check_queued)
    }
}

pub struct RenderGraphBindGroupDescriptor<'g> {
    label: Label<'g>,
    layout: RenderHandle<'g, BindGroupLayout>,
    ///Note: This is not ideal, since we would like to create the dependencies automatically from
    ///the binding list. This isn't possible currently because we'd have to dereference a bind
    ///group layout possibly before it's created. Possible solutions: leave as is, or add Layout information to each entry
    ///so we can infer read/write usage for each binding and maybe create the layout automatically
    ///as well. That might be too verbose though.
    dependencies: RenderDependencies<'g>,
    bindings: Vec<RenderGraphBindGroupEntry<'g>>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RenderGraphBindGroupEntry<'g> {
    pub binding: u32,
    pub resource: RenderGraphBindingResource<'g>,
}

impl<'g> RenderGraphBindGroupEntry<'g> {
    fn as_owned_binding<'a>(&'a self, context: &'a NodeContext<'g>) -> OwnedBindGroupEntry<'a> {
        OwnedBindGroupEntry {
            binding: self.binding,
            resource: self.resource.as_owned_binding(context),
        }
    }
}

struct OwnedBindGroupEntry<'a> {
    binding: u32,
    resource: OwnedGraphBindingResource<'a>,
}

impl<'a> OwnedBindGroupEntry<'a> {
    fn as_binding(&'a self) -> BindGroupEntry<'a> {
        BindGroupEntry {
            binding: self.binding,
            resource: self.resource.as_binding(),
        }
    }
}

enum OwnedGraphBindingResource<'a> {
    Buffer(wgpu::BufferBinding<'a>),
    BufferArray(Vec<wgpu::BufferBinding<'a>>),
    Sampler(&'a wgpu::Sampler),
    SamplerArray(Vec<&'a wgpu::Sampler>),
    TextureView(&'a wgpu::TextureView),
    TextureViewArray(Vec<&'a wgpu::TextureView>),
}

impl<'a> OwnedGraphBindingResource<'a> {
    fn as_binding(&self) -> BindingResource {
        match self {
            OwnedGraphBindingResource::Buffer(buffer) => BindingResource::Buffer(buffer.clone()),
            OwnedGraphBindingResource::BufferArray(buffers) => {
                BindingResource::BufferArray(buffers)
            }
            OwnedGraphBindingResource::Sampler(sampler) => BindingResource::Sampler(sampler),
            OwnedGraphBindingResource::SamplerArray(samplers) => {
                BindingResource::SamplerArray(samplers)
            }
            OwnedGraphBindingResource::TextureView(texture_view) => {
                BindingResource::TextureView(texture_view)
            }
            OwnedGraphBindingResource::TextureViewArray(texture_views) => {
                BindingResource::TextureViewArray(texture_views)
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum RenderGraphBindingResource<'g> {
    Buffer(RenderGraphBufferBinding<'g>),
    BufferArray(Vec<RenderGraphBufferBinding<'g>>),
    Sampler(RenderHandle<'g, Sampler>),
    SamplerArray(Vec<RenderHandle<'g, Sampler>>),
    TextureView(RenderHandle<'g, TextureView>),
    TextureViewArray(Vec<RenderHandle<'g, TextureView>>),
}

impl<'g> RenderGraphBindingResource<'g> {
    fn as_owned_binding<'a>(
        &'a self,
        context: &'a NodeContext<'g>,
    ) -> OwnedGraphBindingResource<'a> {
        match self {
            RenderGraphBindingResource::Buffer(buffer) => {
                OwnedGraphBindingResource::Buffer(buffer.as_owned_binding(context))
            }
            RenderGraphBindingResource::BufferArray(buffers) => {
                let raw_buffers = buffers
                    .iter()
                    .map(|buffer| buffer.as_owned_binding(context))
                    .collect();
                OwnedGraphBindingResource::BufferArray(raw_buffers)
            }
            RenderGraphBindingResource::Sampler(sampler) => {
                OwnedGraphBindingResource::Sampler(context.get(*sampler).deref())
            }
            RenderGraphBindingResource::SamplerArray(samplers) => {
                let raw_samplers = samplers
                    .iter()
                    .map(|sampler| context.get(*sampler).deref())
                    .collect();
                OwnedGraphBindingResource::SamplerArray(raw_samplers)
            }
            RenderGraphBindingResource::TextureView(texture_view) => {
                OwnedGraphBindingResource::TextureView(context.get(*texture_view).deref())
            }
            RenderGraphBindingResource::TextureViewArray(texture_views) => {
                let raw_texture_views = texture_views
                    .iter()
                    .map(|texture_view| context.get(*texture_view).deref())
                    .collect();
                OwnedGraphBindingResource::TextureViewArray(raw_texture_views)
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct RenderGraphBufferBinding<'g> {
    buffer: RenderHandle<'g, Buffer>,
    offset: BufferAddress,
    size: Option<BufferSize>,
}

impl<'g> RenderGraphBufferBinding<'g> {
    fn as_owned_binding<'a>(&'a self, context: &'a NodeContext<'g>) -> BufferBinding<'a> {
        BufferBinding {
            buffer: context.get(self.buffer).deref(),
            offset: self.offset,
            size: self.size,
        }
    }
}

impl RenderResource for BindGroup {
    const RESOURCE_TYPE: ResourceType = ResourceType::BindGroup;

    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.new_bind_group_direct(Default::default(), None, resource)
    }

    fn get_from_store<'a>(
        context: &'a NodeContext<'a>,
        resource: RenderHandle<'a, Self>,
    ) -> Option<&'a Self> {
        context.get_bind_group(resource)
    }
}

impl<'g> IntoRenderResource<'g> for RenderGraphBindGroupDescriptor<'g> {
    type Resource = BindGroup;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_bind_group_descriptor(self)
    }
}
