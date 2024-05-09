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
    NewRenderResource, RenderDependencies, RenderHandle, RenderResource, RenderResourceId,
    ResourceTracker, ResourceType,
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
    bind_groups: HashMap<RenderResourceId, RefEq<'g, BindGroup>>,
    existing_bind_groups: HashMap<RefEq<'g, BindGroup>, RenderResourceId>,
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
        self.existing_bind_groups
            .get(&bind_group)
            .copied()
            .unwrap_or_else(|| {
                let id = tracker.new_resource(ResourceType::BindGroup, dependencies);
                self.existing_bind_groups.insert(bind_group.clone(), id);
                self.bind_groups.insert(id, bind_group);
                id
            })
    }

    pub fn new_from_descriptor(
        &mut self,
        tracker: &mut ResourceTracker<'g>,
        label: Label<'g>,
        layout: RenderHandle<'g, BindGroupLayout>,
        mut dependencies: RenderDependencies<'g>,
        bind_group: impl FnOnce(NodeContext) -> &[BindGroupEntry] + 'g,
    ) -> RenderResourceId {
        dependencies.read(&layout);
        let id = tracker.new_resource(ResourceType::BindGroup, Some(dependencies.clone()));
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
        // view_entity: EntityRef<'g>,
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
                // entity: view_entity,
            };
            let bind_group_entries = (factory)(context.clone());
            let layout = context.get(layout);
            let bind_group = bind_group_cache
                .entry(BindGroupEntriesHash(bind_group_entries))
                .or_insert_with_key(|BindGroupEntriesHash(entries)| {
                    render_device.create_bind_group(label, layout, entries)
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
        // let mut bindings = Vec::from(self.0);
        // bindings.sort_unstable_by_key(|e| e.binding);
        // for entry in self.0 {
        //     entry.binding.hash(state);
        //     match &entry.resource {
        //         wgpu::BindingResource::Buffer(buffer) => {
        //             buffer.buffer.id().hash(state);
        //             buffer.offset.hash(state);
        //             buffer.size.hash(state);
        //         }
        //         wgpu::BindingResource::BufferArray(buffers) => {
        //             for buffer in *buffers {
        //                 buffer.buffer.id().hash(state);
        //                 buffer.offset.hash(state);
        //                 buffer.size.hash(state);
        //             }
        //         }
        //         wgpu::BindingResource::Sampler(sampler) => sampler.id().hash(state),
        //         wgpu::BindingResource::SamplerArray(samplers) => {
        //             for sampler in *samplers {
        //                 sampler.id().hash(state);
        //             }
        //         }
        //         wgpu::BindingResource::TextureView(texture_view) => {
        //             texture_view.id().hash(state);
        //         }
        //         wgpu::BindingResource::TextureViewArray(texture_views) => {
        //             for texture_view in *texture_views {
        //                 texture_view.id().hash(state);
        //             }
        //         }
        //         _ => {}
        //     }
        // }
        todo!()
    }
}

impl<'a> PartialEq for BindGroupEntriesHash<'a> {
    fn eq(&self, other: &Self) -> bool {
        // let mut bindings1 = Vec::from(self.0);
        // let mut bindings2 = Vec::from(other.0);
        //
        // bindings1.sort_unstable_by_key(|e| e.binding);
        // bindings2.sort_unstable_by_key(|e| e.binding);
        //
        // //hacky, since std::iter::eq_by is unstable
        // fn slice_eq_by<A, B>(a: &[A], b: &[B], mut f: impl FnMut(&A, &B) -> bool) -> bool {
        //     a.length() == b.length() && std::iter::zip(a, b).all(|(ai, bi)| f(ai, bi))
        // }
        //
        // slice_eq_by(self.0, other.0, |e1, e2| {
        //     use wgpu::BindingResource as BR;
        //     e1.binding == e2.binding
        //         && match (&e1.resource, &e2.resource) {
        //             (BR::Buffer(b1), BR::Buffer(b2)) => {
        //                 b1.buffer.id() == b2.buffer.id()
        //                     && b1.offset == b2.offset
        //                     && b1.size == b2.size
        //             }
        //             (BR::BufferArray(b1s), BR::BufferArray(b2s)) => {
        //                 slice_eq_by(b1s, b2s, |b1, b2| {
        //                     b1.buffer.id() == b2.buffer.id()
        //                         && b1.offset == b2.offset
        //                         && b1.size == b2.size
        //                 })
        //             }
        //             (BR::Sampler(s1), BR::Sampler(s2)) => s1.id() == s2.id(),
        //             (BR::SamplerArray(s1s), BR::SamplerArray(s2s)) => {
        //                 slice_eq_by(s1s, s2s, |s1, s2| s1.id() == s2.id())
        //             }
        //             (BR::TextureView(t1), BR::TextureView(t2)) => t1.id() == t2.id(),
        //             (BR::TextureViewArray(t1s), BR::TextureViewArray(t2s)) => {
        //                 slice_eq_by(t1s, t2s, |t1, t2| t1.id() == t2.id())
        //             }
        //             _ => false,
        //         }
        // })
        todo!()
    }
}

impl<'a> Eq for BindGroupEntriesHash<'a> {}

impl RenderResource for BindGroup {
    const RESOURCE_TYPE: ResourceType = ResourceType::BindGroup;

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
