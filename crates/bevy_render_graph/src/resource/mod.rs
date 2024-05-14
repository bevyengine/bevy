use std::{borrow::Borrow, collections::VecDeque, fmt::Debug, marker::PhantomData};

use bevy_utils::{HashMap, HashSet};
use std::hash::Hash;

use crate::{render_resource::PipelineCache, renderer::RenderDevice};

use self::ref_eq::RefEq;

use super::{NodeContext, RenderGraph, RenderGraphBuilder};

pub mod bind_group;
pub mod buffer;
pub mod pipeline;
pub(crate) mod ref_eq; //make pub?
pub mod texture;

#[derive(Default)]
pub struct ResourceTracker<'g> {
    next_id: u32,
    resources: Vec<ResourceInfo<'g>>,
}

pub enum ResourceType {
    BindGroupLayout,
    BindGroup,
    Texture,
    TextureView,
    Sampler,
    Buffer,
    RenderPipeline,
    ComputePipeline,
}

struct ResourceInfo<'g> {
    resource_type: ResourceType,
    generation: RenderResourceGeneration,
    dependencies: Option<RenderDependencies<'g>>,
}

impl<'g> ResourceTracker<'g> {
    pub(super) fn new_resource(
        &mut self,
        resource_type: ResourceType,
        dependencies: Option<RenderDependencies<'g>>,
    ) -> RenderResourceId {
        if self.next_id == u32::MAX {
            panic!(
                "No more than {:?} render resources can exist at once across all render graphs",
                u32::MAX
            );
        }
        let id = self.next_id;
        self.next_id += 1;
        self.resources.push(ResourceInfo {
            resource_type,
            generation: 0,
            dependencies,
        });
        RenderResourceId { id }
    }

    pub(super) fn write_dependencies(&mut self, dependencies: &RenderDependencies<'g>) {
        self.collect_many_dependencies(dependencies)
            .writes
            .into_iter()
            .for_each(|id| self.resources[id.id as usize].generation += 1);
    }

    pub(super) fn generation(&self, id: RenderResourceId) -> RenderResourceGeneration {
        self.collect_dependencies(id)
            .iter()
            .map(|id| self.resources[id.id as usize].generation)
            .sum()
    }

    pub(super) fn dependencies_ready(
        &self,
        graph: &RenderGraph<'g>,
        pipeline_cache: &PipelineCache,
        dependencies: &RenderDependencies<'g>,
    ) -> bool {
        let dependencies = self.collect_many_dependencies(dependencies);
        let mut render_dependencies = dependencies.iter();
        render_dependencies.all(|dep| match self.resources[dep.id as usize].resource_type {
            ResourceType::BindGroupLayout => graph.bind_group_layouts.get(dep).is_some(),
            ResourceType::BindGroup => graph.bind_groups.get(dep).is_some(),
            ResourceType::Texture => graph.textures.get(dep).is_some(),
            ResourceType::TextureView => graph.texture_views.get(dep).is_some(),
            ResourceType::Sampler => graph.samplers.get(dep).is_some(),
            ResourceType::Buffer => graph.buffers.get(dep).is_some(),
            ResourceType::RenderPipeline => graph
                .pipelines
                .get_render_pipeline(pipeline_cache, dep)
                .is_some(),
            ResourceType::ComputePipeline => graph
                .pipelines
                .get_compute_pipeline(pipeline_cache, dep)
                .is_some(),
        })
    }

    //There's probably a better way of doing this. Basically flood-filling a graph that may have cycles (but should never really in practice)
    pub(super) fn collect_many_dependencies(
        &self,
        dependencies: &RenderDependencies<'g>,
    ) -> RenderDependencies<'g> {
        let mut new_dependencies = RenderDependencies::new();

        let mut queue = dependencies.iter().collect::<VecDeque<_>>();
        while let Some(new_read) = queue.pop_front() {
            if new_dependencies.reads.contains(&new_read) {
                continue;
            }
            new_dependencies.reads.insert(new_read);
            if let Some(deps) = &self.resources[new_read.id as usize].dependencies {
                queue.extend(deps.iter());
            }
        }

        queue.extend(dependencies.iter_writes());
        while let Some(new_write) = queue.pop_front() {
            if new_dependencies.writes.contains(&new_write) {
                continue;
            }
            new_dependencies.writes.insert(new_write);
            if let Some(deps) = &self.resources[new_write.id as usize].dependencies {
                queue.extend(deps.iter_writes());
            }
        }

        new_dependencies
    }

    fn collect_dependencies(&self, id: RenderResourceId) -> RenderDependencies<'g> {
        self.resources[id.id as usize]
            .dependencies
            .as_ref()
            .map(|deps| self.collect_many_dependencies(deps))
            .unwrap_or_default()
    }
}

pub trait RenderResource: Sized + Send + Sync + 'static {
    const RESOURCE_TYPE: ResourceType;

    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self>;

    fn get_from_store<'a>(
        context: &'a NodeContext<'a>,
        resource: RenderHandle<'a, Self>,
    ) -> Option<&'a Self>;
}

pub trait WriteRenderResource: RenderResource {}

pub trait DescribedRenderResource: RenderResource {
    type Descriptor: Send + Sync + 'static;

    fn new_with_descriptor<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        descriptor: Self::Descriptor,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self>;

    fn get_descriptor<'a, 'g: 'a>(
        graph: &'a RenderGraphBuilder<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a Self::Descriptor>;
}

pub trait FromDescriptorRenderResource: DescribedRenderResource {
    fn new_from_descriptor<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        descriptor: Self::Descriptor,
    ) -> RenderHandle<'g, Self>;
}

pub trait UsagesRenderResource: DescribedRenderResource {
    type Usages: Send + Sync + Debug + 'static;

    fn get_descriptor_mut<'b, 'g: 'b>(
        graph: &'b mut RenderGraphBuilder<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'b mut Self::Descriptor>;

    fn has_usages(descriptor: &Self::Descriptor, usages: &Self::Usages) -> bool;
    fn add_usages(descriptor: &mut Self::Descriptor, usages: Self::Usages);
}

struct RenderResourceMeta<'g, R: DescribedRenderResource> {
    pub descriptor: Option<R::Descriptor>,
    pub resource: RefEq<'g, R>,
}

impl<'g, R: DescribedRenderResource + Clone> Clone for RenderResourceMeta<'g, R>
where
    R::Descriptor: Clone,
{
    fn clone(&self) -> Self {
        Self {
            descriptor: self.descriptor.clone(),
            resource: self.resource.clone(),
        }
    }
}

pub enum NewRenderResource<'g, R: FromDescriptorRenderResource> {
    FromDescriptor(R::Descriptor),
    Resource(Option<R::Descriptor>, RefEq<'g, R>),
}

pub struct RenderResources<'g, R: DescribedRenderResource> {
    resources: HashMap<RenderResourceId, RenderResourceMeta<'g, R>>,
    existing_resources: HashMap<RefEq<'g, R>, RenderResourceId>,
    queued_resources: HashMap<RenderResourceId, R::Descriptor>,
    resource_factory: Box<dyn Fn(&RenderDevice, &R::Descriptor) -> R>,
}

impl<'g, R: DescribedRenderResource> RenderResources<'g, R> {
    pub fn new(factory: impl Fn(&RenderDevice, &R::Descriptor) -> R + 'static) -> Self {
        Self {
            resources: Default::default(),
            existing_resources: Default::default(),
            queued_resources: Default::default(),
            resource_factory: Box::new(factory),
        }
    }

    pub fn new_direct(
        &mut self,
        tracker: &mut ResourceTracker,
        descriptor: Option<R::Descriptor>,
        resource: RefEq<'g, R>,
    ) -> RenderResourceId
    where
        RefEq<'g, R>: Clone + Hash + Eq,
    {
        self.existing_resources
            .get(&resource)
            .copied()
            .unwrap_or_else(|| {
                let id = tracker.new_resource(R::RESOURCE_TYPE, None);
                self.existing_resources.insert(resource.clone(), id);
                self.resources.insert(
                    id,
                    RenderResourceMeta {
                        descriptor,
                        resource,
                    },
                );
                id
            })
    }

    pub fn new_from_descriptor(
        &mut self,
        tracker: &mut ResourceTracker,
        descriptor: R::Descriptor,
    ) -> RenderResourceId {
        let id = tracker.new_resource(R::RESOURCE_TYPE, None);
        self.queued_resources.insert(id, descriptor);
        id
    }

    pub fn create_queued_resources(&mut self, render_device: &RenderDevice) {
        for (id, descriptor) in self.queued_resources.drain() {
            let resource = (self.resource_factory)(render_device, &descriptor);
            self.resources.insert(
                id,
                RenderResourceMeta {
                    descriptor: Some(descriptor),
                    resource: RefEq::Owned(resource),
                },
            );
        }
    }

    pub fn create_queued_resources_cached(
        &mut self,
        cache: &mut CachedResources<R>,
        render_device: &RenderDevice,
    ) where
        R::Descriptor: Clone + Hash + Eq,
    {
        for (_, descriptor) in &self.queued_resources {
            cache
                .cached_resources
                .entry(descriptor.clone())
                .or_insert_with(|| (self.resource_factory)(render_device, descriptor));
        }
    }

    pub fn borrow_cached_resources(&mut self, cache: &'g CachedResources<R>)
    where
        R::Descriptor: Clone + Hash + Eq,
    {
        for (id, descriptor) in self.queued_resources.drain() {
            if let Some(resource) = cache.cached_resources.get(&descriptor) {
                self.resources.insert(
                    id,
                    RenderResourceMeta {
                        descriptor: Some(descriptor),
                        resource: RefEq::Borrowed(resource),
                    },
                );
            }
        }
    }

    pub fn get_descriptor(&self, id: RenderResourceId) -> Option<&R::Descriptor> {
        let check_normal = self
            .resources
            .get(&id)
            .and_then(|meta| meta.descriptor.as_ref());
        let check_queued = self.queued_resources.get(&id);
        check_normal.or(check_queued)
    }

    pub fn get(&self, id: RenderResourceId) -> Option<&R> {
        self.resources.get(&id).map(|meta| meta.resource.borrow())
    }
}

impl<'g, R: UsagesRenderResource> RenderResources<'g, R> {
    pub fn get_descriptor_mut(&mut self, id: RenderResourceId) -> Option<&mut R::Descriptor> {
        self.queued_resources.get_mut(&id)
    }

    pub fn add_usages(&mut self, id: RenderResourceId, usages: R::Usages) {
        if let Some(descriptor) = self.queued_resources.get_mut(&id) {
            R::add_usages(descriptor, usages);
        }
    }
}

pub struct CachedResources<R: DescribedRenderResource>
where
    R::Descriptor: Clone + Hash + Eq,
{
    cached_resources: HashMap<R::Descriptor, R>,
}

impl<R: DescribedRenderResource> Default for CachedResources<R>
where
    R::Descriptor: Clone + Hash + Eq,
{
    fn default() -> Self {
        Self {
            cached_resources: Default::default(),
        }
    }
}

pub trait IntoRenderResource<'g> {
    type Resource: RenderResource;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource>;
}

impl<'g, R: RenderResource, F: FnOnce(&RenderDevice) -> R> IntoRenderResource<'g> for F {
    type Resource = R;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        let resource = (self)(graph.render_device);
        graph.into_resource(resource)
    }
}

impl<'g, R: FromDescriptorRenderResource> IntoRenderResource<'g> for NewRenderResource<'g, R> {
    type Resource = R;

    fn into_render_resource(self, graph: &mut RenderGraphBuilder<'g>) -> RenderHandle<'g, R> {
        match self {
            NewRenderResource::FromDescriptor(descriptor) => {
                R::new_from_descriptor(graph, descriptor)
            }

            NewRenderResource::Resource(Some(descriptor), resource) => {
                R::new_with_descriptor(graph, descriptor, resource)
            }
            NewRenderResource::Resource(None, resource) => R::new_direct(graph, resource),
        }
    }
}

impl<'g, R: RenderResource> IntoRenderResource<'g> for RefEq<'g, R> {
    type Resource = R;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        R::new_direct(graph, self)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct RenderResourceId {
    id: u32,
}

pub type RenderResourceGeneration = u16;

pub struct RenderHandle<'g, R: RenderResource> {
    id: RenderResourceId,
    data: PhantomData<&'g R>,
}

impl<'g, R: RenderResource> PartialEq for RenderHandle<'g, R> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<'g, R: RenderResource> Eq for RenderHandle<'g, R> {}

impl<'g, R: RenderResource> Hash for RenderHandle<'g, R> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<'g, R: RenderResource> Copy for RenderHandle<'g, R> {}

impl<'g, R: RenderResource> Clone for RenderHandle<'g, R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'g, R: RenderResource> Debug for RenderHandle<'g, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderHandle")
            .field("id", &self.id)
            .finish()
    }
}

impl<'g, R: RenderResource> RenderHandle<'g, R> {
    pub(super) fn new(id: RenderResourceId) -> Self {
        Self {
            id,
            data: PhantomData,
        }
    }

    pub(super) fn id(&self) -> RenderResourceId {
        self.id
    }
}

#[derive(Default, PartialEq, Eq, Clone)]
pub struct RenderDependencies<'g> {
    reads: HashSet<RenderResourceId>,
    writes: HashSet<RenderResourceId>,
    data: PhantomData<RenderGraph<'g>>,
}

impl<'g> RenderDependencies<'g> {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    #[inline]
    pub fn read<R: RenderResource>(&mut self, resource: RenderHandle<'g, R>) -> &mut Self {
        self.reads.insert(resource.id());
        self
    }

    #[inline]
    pub fn write<R: WriteRenderResource>(&mut self, resource: RenderHandle<'g, R>) -> &mut Self {
        self.writes.insert(resource.id());
        self
    }

    #[inline]
    pub fn add(&mut self, dependencies: impl IntoRenderDependencies<'g>) -> &mut Self {
        dependencies.into_render_dependencies(self);
        self
    }

    #[inline]
    pub fn extend(&mut self, other: RenderDependencies<'g>) -> &mut Self {
        self.reads.extend(other.iter_reads());
        self.writes.extend(other.iter_writes());
        self
    }

    #[inline]
    pub fn reads<R: RenderResource>(&self, resource: RenderHandle<'g, R>) -> bool {
        self.reads.contains(&resource.id())
    }

    #[inline]
    pub fn writes<R: RenderResource>(&self, resource: RenderHandle<'g, R>) -> bool {
        self.writes.contains(&resource.id())
    }

    #[inline]
    pub fn includes<R: RenderResource>(&self, resource: RenderHandle<'g, R>) -> bool {
        self.reads(resource) || self.writes(resource)
    }

    #[inline]
    pub(super) fn iter_reads(&self) -> impl Iterator<Item = RenderResourceId> + '_ {
        self.reads.iter().copied()
    }

    #[inline]
    pub(super) fn iter_writes(&self) -> impl Iterator<Item = RenderResourceId> + '_ {
        self.writes.iter().copied()
    }

    #[inline]
    pub(super) fn iter(&self) -> impl Iterator<Item = RenderResourceId> + '_ {
        self.iter_reads().chain(self.iter_writes())
    }
}

impl<'g, R: RenderResource> IntoRenderDependencies<'g> for &RenderHandle<'g, R> {
    #[inline]
    fn into_render_dependencies(self, dependencies: &mut RenderDependencies<'g>) {
        dependencies.read(*self);
    }
}

impl<'g, R: WriteRenderResource> IntoRenderDependencies<'g> for &mut RenderHandle<'g, R> {
    #[inline]
    fn into_render_dependencies(self, dependencies: &mut RenderDependencies<'g>) {
        dependencies.write(*self);
    }
}

pub trait IntoRenderDependencies<'g> {
    fn into_render_dependencies(self, dependencies: &mut RenderDependencies<'g>);
}

//in macro crate?
#[macro_export]
macro_rules! deps {
    ($($dep: expr),*) => {
        {
            let mut dependencies = $crate::render_graph_v2::resource::RenderDependencies::new();
            $crate::render_graph_v2::resource::extend_deps!(dependencies, $($dep),*);
            dependencies
        }
    }
}

pub use deps;

#[macro_export]
macro_rules! extend_deps {
    ($dependencies: ident, $($dep: expr),*) => {
        {
            $($dependencies.add($dep);)*
        }
    }
}

pub use extend_deps;
