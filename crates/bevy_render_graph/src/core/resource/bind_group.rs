use std::{
    borrow::Cow,
    hash::Hash,
    ops::{Deref, Range},
    rc::Rc,
};

use bevy_render::{
    render_resource::{
        BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry, BindingResource,
        BindingType, Buffer, BufferAddress, BufferBinding, BufferBindingType, BufferSize, Sampler,
        StorageTextureAccess, TextureView,
    },
    renderer::RenderDevice,
};
use bevy_utils::HashSet;

use crate::core::{Label, NodeContext, RenderGraphBuilder};

use super::{
    CacheRenderResource, IntoRenderResource, RenderDependencies, RenderHandle, RenderResource,
    ResourceType, WriteRenderResource,
};

#[derive(Clone)]
pub struct RenderGraphBindGroupLayoutMeta {
    pub descriptor: RenderGraphBindGroupLayoutDescriptor,
    writes: Rc<HashSet<u32>>,
}

impl RenderGraphBindGroupLayoutMeta {
    pub fn new(descriptor: RenderGraphBindGroupLayoutDescriptor) -> Self {
        let mut writes = HashSet::new();
        for entry in &descriptor.entries {
            let writes_entry = match entry.ty {
                BindingType::Buffer { ty, .. } => match ty {
                    BufferBindingType::Uniform => false,
                    BufferBindingType::Storage { read_only } => !read_only,
                },
                BindingType::Sampler(_) => false,
                BindingType::Texture { .. } => false,
                BindingType::StorageTexture { access, .. } => match access {
                    StorageTextureAccess::WriteOnly => true,
                    StorageTextureAccess::ReadOnly => false,
                    StorageTextureAccess::ReadWrite => true,
                },
                BindingType::AccelerationStructure => false,
            };
            if writes_entry {
                writes.insert(entry.binding);
            }
        }

        RenderGraphBindGroupLayoutMeta {
            descriptor,
            writes: Rc::new(writes),
        }
    }
}

#[derive(Clone)]
pub struct RenderGraphBindGroupLayoutDescriptor {
    pub label: Label<'static>,
    pub entries: Vec<BindGroupLayoutEntry>,
}

impl Hash for RenderGraphBindGroupLayoutDescriptor {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.entries.hash(state);
    }
}

impl PartialEq for RenderGraphBindGroupLayoutDescriptor {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl Eq for RenderGraphBindGroupLayoutDescriptor {}

impl RenderResource for BindGroupLayout {
    const RESOURCE_TYPE: ResourceType = ResourceType::BindGroupLayout;
    type Meta<'g> = RenderGraphBindGroupLayoutMeta;

    #[inline]
    fn import<'g>(
        graph: &mut RenderGraphBuilder<'_, 'g>,
        meta: Self::Meta<'g>,
        resource: Cow<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.import_bind_group_layout(meta, resource)
    }

    #[inline]
    fn get<'n, 'g: 'n>(
        context: &'n NodeContext<'n, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'n Self> {
        context.get_bind_group_layout(resource)
    }

    #[inline]
    fn get_meta<'a, 'b: 'a, 'g: 'b>(
        graph: &'a RenderGraphBuilder<'b, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a Self::Meta<'g>> {
        graph.get_bind_group_layout_meta(resource)
    }
}

impl CacheRenderResource for BindGroupLayout {
    type Key = RenderGraphBindGroupLayoutDescriptor;

    #[inline]
    fn key_from_meta<'a, 'g: 'a>(meta: &'a Self::Meta<'g>) -> &'a Self::Key {
        &meta.descriptor
    }
}

impl<'g> IntoRenderResource<'g> for RenderGraphBindGroupLayoutDescriptor {
    type Resource = BindGroupLayout;

    #[inline]
    fn into_render_resource(
        mut self,
        graph: &mut RenderGraphBuilder<'_, 'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        self.entries.sort_by_key(|entry| entry.binding);
        graph.new_bind_group_layout(RenderGraphBindGroupLayoutMeta::new(self))
    }
}

impl<'g> IntoRenderResource<'g> for Vec<BindGroupLayoutEntry> {
    type Resource = BindGroupLayout;

    #[inline]
    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'_, 'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_resource(RenderGraphBindGroupLayoutDescriptor {
            label: None,
            entries: self,
        })
    }
}

impl<'g> IntoRenderResource<'g> for &[BindGroupLayoutEntry] {
    type Resource = BindGroupLayout;

    #[inline]
    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'_, 'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_resource(Vec::from(self))
    }
}

pub(crate) fn make_bind_group<'n, 'g: 'n>(
    ctx: &NodeContext<'n, 'g>,
    render_device: &RenderDevice,
    descriptor: &RenderGraphBindGroupDescriptor<'g>,
) -> BindGroup {
    fn push_ref<T>(vec: &mut Vec<T>, item: T) -> Range<usize> {
        let i_fst = vec.len();
        vec.push(item);
        i_fst..vec.len()
    }

    fn push_many_ref<T>(vec: &mut Vec<T>, items: impl IntoIterator<Item = T>) -> Range<usize> {
        let i_fst = vec.len();
        vec.extend(items);
        i_fst..vec.len()
    }

    let ctx_ref = &ctx;

    let raw_layout = ctx.get(descriptor.layout);

    let mut buffers = Vec::with_capacity(descriptor.entries.len());
    let mut samplers = Vec::with_capacity(descriptor.entries.len());
    let mut texture_views = Vec::with_capacity(descriptor.entries.len());
    let mut indices: Vec<Range<usize>> = Vec::with_capacity(descriptor.entries.len());

    descriptor.entries.iter().enumerate().for_each(
        |(i, RenderGraphBindGroupEntry { resource, .. })| {
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
        },
    );

    let raw_entries: Vec<BindGroupEntry<'_>> = descriptor
        .entries
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

    render_device.create_bind_group(descriptor.label.as_deref(), raw_layout, &raw_entries)
}

#[derive(Clone)]
pub struct RenderGraphBindGroupMeta<'g> {
    pub descriptor: RenderGraphBindGroupDescriptor<'g>,
    writes: Rc<HashSet<u32>>,
}

impl<'g> RenderGraphBindGroupMeta<'g> {
    pub fn new(
        graph: &mut RenderGraphBuilder<'_, 'g>,
        descriptor: RenderGraphBindGroupDescriptor<'g>,
    ) -> Self {
        let layout_meta = graph.meta(descriptor.layout);
        Self {
            descriptor,
            writes: layout_meta.writes.clone(),
        }
    }

    pub(crate) fn dependencies(&self) -> RenderDependencies<'g> {
        fn dep<'g, R: WriteRenderResource>(
            deps: &mut RenderDependencies<'g>,
            resource: RenderHandle<'g, R>,
            is_mut: bool,
        ) {
            if is_mut {
                deps.write(resource);
            } else {
                deps.read(resource);
            }
        }

        fn deps<'g, R: WriteRenderResource>(
            deps: &mut RenderDependencies<'g>,
            resources: impl Iterator<Item = RenderHandle<'g, R>>,
            is_mut: bool,
        ) {
            if is_mut {
                for resource in resources {
                    deps.write(resource);
                }
            } else {
                for resource in resources {
                    deps.read(resource);
                }
            }
        }

        let mut dependencies = RenderDependencies::new();
        for entry in &self.descriptor.entries {
            let write = self.writes.contains(&entry.binding);
            match &entry.resource {
                RenderGraphBindingResource::Buffer(buffer) => {
                    dep(&mut dependencies, buffer.buffer, write)
                }
                RenderGraphBindingResource::BufferArray(buffers) => deps(
                    &mut dependencies,
                    buffers.iter().map(|buffer| buffer.buffer),
                    write,
                ),
                RenderGraphBindingResource::Sampler(sampler_binding) => {
                    dependencies.read(*sampler_binding);
                }
                RenderGraphBindingResource::SamplerArray(samplers) => {
                    for sampler in samplers.iter() {
                        dependencies.read(*sampler);
                    }
                }
                RenderGraphBindingResource::TextureView(texture_view) => {
                    dep(&mut dependencies, *texture_view, write)
                }
                RenderGraphBindingResource::TextureViewArray(texture_views) => {
                    deps(&mut dependencies, texture_views.iter().copied(), write)
                }
            }
        }

        dependencies
    }
}

#[derive(Clone)]
pub struct RenderGraphBindGroupDescriptor<'g> {
    pub label: Label<'g>,
    pub layout: RenderHandle<'g, BindGroupLayout>,
    pub entries: Vec<RenderGraphBindGroupEntry<'g>>,
}

impl<'g> Hash for RenderGraphBindGroupDescriptor<'g> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.layout.hash(state);
        self.entries.hash(state);
    }
}

impl<'g> PartialEq for RenderGraphBindGroupDescriptor<'g> {
    fn eq(&self, other: &Self) -> bool {
        self.layout == other.layout && self.entries == other.entries
    }
}

impl<'g> Eq for RenderGraphBindGroupDescriptor<'g> {}

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
    type Meta<'g> = RenderGraphBindGroupMeta<'g>;

    fn import<'g>(
        graph: &mut RenderGraphBuilder<'_, 'g>,
        meta: Self::Meta<'g>,
        resource: Cow<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.import_bind_group(meta, resource)
    }

    fn get<'n, 'g: 'n>(
        context: &'n NodeContext<'n, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'n Self> {
        context.get_bind_group(resource)
    }

    fn get_meta<'a, 'b: 'a, 'g: 'b>(
        graph: &'a RenderGraphBuilder<'b, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a Self::Meta<'g>> {
        graph.get_bind_group_meta(resource)
    }
}

impl WriteRenderResource for BindGroup {}

impl<'g> IntoRenderResource<'g> for RenderGraphBindGroupDescriptor<'g> {
    type Resource = BindGroup;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'_, 'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        let meta = RenderGraphBindGroupMeta::new(graph, self);
        graph.new_bind_group(meta)
    }
}
