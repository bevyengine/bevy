use std::{
    borrow::Borrow,
    hash::Hash,
    ops::{Deref, Range},
};

use bevy_ecs::world::World;
use bevy_utils::HashMap;

use bevy_render::{
    render_resource::{
        BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry, BindingResource, Buffer,
        BufferAddress, BufferBinding, BufferSize, Sampler, TextureView,
    },
    renderer::RenderDevice,
};

use crate::core::{Label, NodeContext, RenderGraph, RenderGraphBuilder};

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

impl<'g> IntoRenderResource<'g> for Vec<BindGroupLayoutEntry> {
    type Resource = BindGroupLayout;

    #[inline]
    fn into_render_resource(
        mut self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        self.sort_by_key(|entry| entry.binding);
        graph.new_bind_group_layout_descriptor(self)
    }
}

impl<'g> IntoRenderResource<'g> for &[BindGroupLayoutEntry] {
    type Resource = BindGroupLayout;

    #[inline]
    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_resource(Vec::from(self))
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
                mut bindings,
            },
        ) in self.queued_bind_groups.drain()
        {
            let context = NodeContext {
                graph,
                world,
                dependencies,
                // entity: view_entity,
            };

            bindings.sort_by_key(|entry| entry.binding);
            let bind_group = bind_group_cache
                .entry(bindings)
                .or_insert_with_key(|bindings| {
                    make_bind_group(context, render_device, label, layout, bindings)
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

fn make_bind_group<'n>(
    ctx: NodeContext<'n>,
    render_device: &RenderDevice,
    label: Label<'n>,
    layout: RenderHandle<'n, BindGroupLayout>,
    entries: &[RenderGraphBindGroupEntry<'n>],
) -> BindGroup {
    #[inline]
    fn push_ref<T>(vec: &mut Vec<T>, item: T) -> Range<usize> {
        let i_fst = vec.len();
        vec.push(item);
        i_fst..vec.len()
    }

    #[inline]
    fn push_many_ref<T>(vec: &mut Vec<T>, items: impl IntoIterator<Item = T>) -> Range<usize> {
        let i_fst = vec.len();
        vec.extend(items);
        i_fst..vec.len()
    }

    let ctx_ref = &ctx;

    let raw_layout = ctx.get(layout);

    let mut buffers = Vec::with_capacity(entries.len());
    let mut samplers = Vec::with_capacity(entries.len());
    let mut texture_views = Vec::with_capacity(entries.len());
    let mut indices: Vec<Range<usize>> = Vec::with_capacity(entries.len());

    entries
        .iter()
        .enumerate()
        .for_each(|(i, RenderGraphBindGroupEntry { resource, .. })| {
            indices[i] = match resource {
                RenderGraphBindingResource::Buffer(buffer_binding) => push_ref(
                    &mut buffers,
                    BufferBinding {
                        buffer: ctx_ref.get(buffer_binding.buffer).deref(),
                        offset: buffer_binding.offset,
                        size: buffer_binding.size,
                    },
                ),
                RenderGraphBindingResource::BufferArray(buffer_bindings) => push_many_ref(
                    &mut buffers,
                    buffer_bindings.iter().map(|buffer_binding| BufferBinding {
                        buffer: ctx_ref.get(buffer_binding.buffer).deref(),
                        offset: buffer_binding.offset,
                        size: buffer_binding.size,
                    }),
                ),
                RenderGraphBindingResource::Sampler(sampler_binding) => {
                    push_ref(&mut samplers, ctx_ref.get(*sampler_binding).deref())
                }
                RenderGraphBindingResource::SamplerArray(sampler_bindings) => push_many_ref(
                    &mut samplers,
                    sampler_bindings
                        .iter()
                        .map(|sampler| ctx_ref.get(*sampler).deref()),
                ),
                RenderGraphBindingResource::TextureView(texture_view_binding) => push_ref(
                    &mut texture_views,
                    ctx_ref.get(*texture_view_binding).deref(),
                ),
                RenderGraphBindingResource::TextureViewArray(texture_view_bindings) => {
                    push_many_ref(
                        &mut texture_views,
                        texture_view_bindings
                            .iter()
                            .map(|texture_view| ctx_ref.get(*texture_view).deref()),
                    )
                }
            };
        });

    let raw_entries: Vec<BindGroupEntry<'_>> = entries
        .iter()
        .enumerate()
        .map(
            |(i, RenderGraphBindGroupEntry { binding, resource })| BindGroupEntry {
                binding: *binding,
                resource: match resource {
                    RenderGraphBindingResource::Buffer(_) => {
                        BindingResource::Buffer(buffers[indices[i].start].clone())
                    }
                    RenderGraphBindingResource::BufferArray(_) => {
                        BindingResource::BufferArray(&buffers[indices[i].clone()])
                    }
                    RenderGraphBindingResource::Sampler(_) => {
                        BindingResource::Sampler(samplers[indices[i].start])
                    }
                    RenderGraphBindingResource::SamplerArray(_) => {
                        BindingResource::SamplerArray(&samplers[indices[i].clone()])
                    }
                    RenderGraphBindingResource::TextureView(_) => {
                        BindingResource::TextureView(texture_views[indices[i].start])
                    }
                    RenderGraphBindingResource::TextureViewArray(_) => {
                        BindingResource::TextureViewArray(&texture_views[indices[i].clone()])
                    }
                },
            },
        )
        .collect();

    render_device.create_bind_group(label, raw_layout, &raw_entries)
}

pub struct RenderGraphBindGroupDescriptor<'g> {
    pub label: Label<'g>,
    pub layout: RenderHandle<'g, BindGroupLayout>,
    ///Note: This is not ideal, since we would like to create the dependencies automatically from
    ///the binding list. This isn't possible currently because we'd have to dereference a bind
    ///group layout possibly before it's created. Possible solutions: leave as is, or add Layout information to each entry
    ///so we can infer read/write usage for each binding and maybe create the layout automatically
    ///as well. That might be too verbose though.
    pub dependencies: RenderDependencies<'g>,
    pub bindings: Vec<RenderGraphBindGroupEntry<'g>>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RenderGraphBindGroupEntry<'g> {
    pub binding: u32,
    pub resource: RenderGraphBindingResource<'g>,
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

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct RenderGraphBufferBinding<'g> {
    buffer: RenderHandle<'g, Buffer>,
    offset: BufferAddress,
    size: Option<BufferSize>,
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
