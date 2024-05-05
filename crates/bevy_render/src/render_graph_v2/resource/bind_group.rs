use std::{borrow::Borrow, hash::Hash};

use bevy_ecs::world::{EntityRef, World};
use bevy_utils::HashMap;
use encase::rts_array::Length;
use wgpu::{BindGroupEntry, BindGroupLayoutEntry, BufferBinding, Label};

use crate::{
    render_graph_v2::{NodeContext, RenderGraph, RenderGraphBuilder},
    render_resource::{AsBindGroup, BindGroup, BindGroupLayout},
    renderer::RenderDevice,
};

use super::{
    ref_eq::RefEq, DescribedRenderResource, FromDescriptorRenderResource, IntoRenderResource,
    RenderDependencies, RenderHandle, RenderResource, RenderResourceId, ResourceTracker,
};

impl RenderResource for BindGroupLayout {
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
    type Descriptor = Box<[BindGroupLayoutEntry]>;

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

#[derive(Default)]
pub struct RenderGraphBindGroups<'g> {
    bind_groups: HashMap<RenderResourceId, RefEq<'g, BindGroup>>,
    existing_borrows: HashMap<*const BindGroup, RenderResourceId>,
    queued_bind_groups: HashMap<RenderResourceId, QueuedBindGroup<'g>>,
}

struct QueuedBindGroup<'g> {
    label: Label<'g>,
    layout: RenderHandle<'g, BindGroupLayout>,
    dependencies: RenderDependencies<'g>,
    factory: Box<dyn FnOnce(NodeContext) -> &[BindGroupEntry] + 'g>,
}

impl<'g> RenderGraphBindGroups<'g> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_direct(
        &mut self,
        tracker: &mut ResourceTracker<'g>,
        dependencies: Option<RenderDependencies<'g>>,
        bind_group: RefEq<'g, BindGroup>,
    ) -> RenderResourceId {
        match bind_group {
            RefEq::Borrowed(bind_group) => {
                if let Some(id) = self.existing_borrows.get(&(bind_group as *const BindGroup)) {
                    *id
                } else {
                    let id = tracker.new_resource(dependencies);
                    self.bind_groups.insert(id, RefEq::Borrowed(bind_group));
                    self.existing_borrows
                        .insert(bind_group as *const BindGroup, id);
                    id
                }
            }
            RefEq::Owned(_) => {
                let id = tracker.new_resource(dependencies);
                self.bind_groups.insert(id, bind_group);
                id
            }
        }
    }

    pub fn new_from_descriptor(
        &mut self,
        tracker: &mut ResourceTracker<'g>,
        label: Label<'g>,
        layout: RenderHandle<'g, BindGroupLayout>,
        mut dependencies: RenderDependencies<'g>,
        bind_group: impl FnOnce(NodeContext) -> &[BindGroupEntry] + 'g,
    ) -> RenderResourceId {
        dependencies.add(&layout);
        let id = tracker.new_resource(Some(dependencies.clone()));
        self.queued_bind_groups.insert(
            id,
            QueuedBindGroup {
                label,
                layout,
                dependencies,
                factory: Box::new(bind_group),
            },
        );
        id
    }

    pub fn create_queued_bind_groups(
        &mut self,
        graph: &RenderGraph,
        world: &World,
        render_device: &RenderDevice,
        view_entity: EntityRef<'g>,
    ) {
        let mut bind_group_cache = HashMap::new();
        for (
            id,
            QueuedBindGroup {
                dependencies,
                label,
                layout,
                factory,
            },
        ) in self.queued_bind_groups.drain()
        {
            let context = NodeContext {
                graph,
                world,
                dependencies,
                view_entity,
            };
            let bind_group_entries = (factory)(context);
            let layout = context.get(layout);
            let bind_group = bind_group_cache
                .entry(BindGroupEntriesHash(bind_group_entries))
                .or_insert_with_key(|BindGroupEntriesHash(entries)| {
                    render_device.create_bind_group(label, layout, *entries)
                });
            self.bind_groups
                .insert(id, RefEq::Owned(bind_group.clone()));
        }
        todo!()
    }

    pub fn get(&self, id: RenderResourceId) -> Option<&BindGroup> {
        self.bind_groups.get(&id).map(Borrow::borrow)
    }
}

struct BindGroupEntriesHash<'a>(&'a [BindGroupEntry<'a>]);

impl<'a> Hash for BindGroupEntriesHash<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for entry in self.0 {
            entry.binding.hash(state);
            match entry.resource {
                wgpu::BindingResource::Buffer(BufferBinding {
                    buffer,
                    offset,
                    size,
                }) => {
                    buffer.global_id().hash(state);
                    offset.hash(state);
                    size.hash(state);
                }
                wgpu::BindingResource::BufferArray(_) => todo!(),
                wgpu::BindingResource::Sampler(sampler) => sampler.global_id().hash(state),
                wgpu::BindingResource::SamplerArray(_) => todo!(),
                wgpu::BindingResource::TextureView(texture_view) => {
                    texture_view.global_id().hash(state)
                }
                wgpu::BindingResource::TextureViewArray(_) => todo!(),
                _ => todo!(),
            }
        }
    }
}

impl<'a> PartialEq for BindGroupEntriesHash<'a> {
    fn eq(&self, other: &Self) -> bool {
        if self.0.length() != other.0.length() {
            return false;
        }
        use wgpu::BindingResource as BR;
        for (e1, e2) in std::iter::zip(self.0, other.0) {
            match (e1, e2) {}
        }
        true
    }
}

impl<'a> Eq for BindGroupEntriesHash<'a> {}

impl RenderResource for BindGroup {
    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.new_bind_group_direct(Default::default(), resource)
    }

    fn get_from_store<'a>(
        context: &'a NodeContext<'a>,
        resource: RenderHandle<'a, Self>,
    ) -> Option<&'a Self> {
        todo!()
    }
}

pub struct RenderGraphBindGroup<'g, F: FnOnce(NodeContext<'g>) -> &'g [BindGroupEntry<'g>] + 'g> {
    label: Label<'g>,
    layout: RenderHandle<'g, BindGroupLayout>,
    bind_group: F,
}

impl<'g, F: FnOnce(NodeContext<'g>) -> &'g [BindGroupEntry<'g>] + 'g> IntoRenderResource<'g>
    for RenderGraphBindGroup<'g, F>
{
    type Resource = BindGroup;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        todo!()
    }
}

pub struct AsRenderGraphBindGroup<'g, T: AsBindGroup, F: FnOnce(NodeContext<'g>) -> T + 'g> {
    label: Label<'g>,
    bind_group: F,
}

impl<'g, T: AsBindGroup, F: FnOnce(NodeContext<'g>) -> T + 'g> IntoRenderResource<'g>
    for AsRenderGraphBindGroup<'g, T, F>
{
    type Resource = BindGroup;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        todo!()
    }
}
