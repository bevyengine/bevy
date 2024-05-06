use std::{borrow::Borrow, fmt::Debug, marker::PhantomData};

use bevy_utils::{all_tuples, HashMap, HashSet};
use std::hash::Hash;

use crate::{render_resource::PipelineCache, renderer::RenderDevice};

use self::ref_eq::RefEq;

use super::{NodeContext, RenderGraphBuilder, RenderGraphExecution};

pub mod bind_group;
pub mod buffer;
pub mod pipeline;
pub(crate) mod ref_eq;
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
    pub(super) fn clear(&mut self) {
        self.next_id = 0;
        self.resources.clear();
    }

    pub(super) fn new_resource(
        &mut self,
        resource_type: ResourceType,
        dependencies: Option<RenderDependencies<'g>>,
    ) -> RenderResourceId {
        //TODO: IMPORTANT: debug check for dependency cycles
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

    pub(super) fn write_dependencies(&mut self, dependencies: RenderDependencies<'g>) {
        //NOTE: takes dependencies instead of single resource in order to deduplicate writes in
        //same "set"
        // self.collect_dependencies(id)
        //     .writes
        //     .into_iter()
        //     .for_each(|id| self.generations[id.id as usize].generation += 1);
    }

    pub(super) fn generation(&self, id: RenderResourceId) -> RenderResourceGeneration {
        self.collect_dependencies(id)
            .iter()
            .map(|id| self.resources[id.id as usize].generation)
            .sum()
    }

    pub(super) fn dependencies_ready(
        &self,
        graph: &RenderGraphExecution<'g>,
        pipeline_cache: &PipelineCache,
        dependencies: RenderDependencies<'g>,
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

    fn collect_many_dependencies(
        &self,
        dependencies: RenderDependencies<'g>,
    ) -> RenderDependencies<'g> {
        // let mut dependencies = self.generations[id.id as usize]
        //     .dependencies
        //     .clone()
        //     .unwrap_or_default();
        // //TODO: THIS IS IMPORTANT
        // todo!();
        // dependencies
        todo!()
    }

    fn collect_dependencies(&self, id: RenderResourceId) -> RenderDependencies<'g> {
        todo!()
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

    fn get_descriptor_mut<'a, 'g: 'a>(
        graph: &'a mut RenderGraphBuilder<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a mut Self::Descriptor>;

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

pub enum NewRenderResource<'g, R: DescribedRenderResource> {
    FromDescriptor(R::Descriptor),
    Resource(Option<R::Descriptor>, RefEq<'g, R>),
}

pub struct RenderResources<'g, R: DescribedRenderResource> {
    resources: HashMap<RenderResourceId, RenderResourceMeta<'g, R>>,
    existing_borrows: HashMap<*const R, RenderResourceId>,
    queued_resources: HashMap<RenderResourceId, R::Descriptor>,
    resource_factory: Box<dyn Fn(&RenderDevice, &R::Descriptor) -> R>,
}

impl<'g, R: DescribedRenderResource> RenderResources<'g, R> {
    pub fn new(factory: impl Fn(&RenderDevice, &R::Descriptor) -> R + 'static) -> Self {
        Self {
            resources: Default::default(),
            existing_borrows: Default::default(),
            queued_resources: Default::default(),
            resource_factory: Box::new(factory),
        }
    }

    pub fn new_direct(
        &mut self,
        tracker: &mut ResourceTracker,
        descriptor: Option<R::Descriptor>,
        resource: RefEq<'g, R>,
    ) -> RenderResourceId {
        match resource {
            RefEq::Borrowed(resource) => {
                if let Some(id) = self.existing_borrows.get(&(resource as *const R)) {
                    *id
                } else {
                    let id = tracker.new_resource(R::RESOURCE_TYPE, None);
                    self.resources.insert(
                        id,
                        RenderResourceMeta {
                            descriptor,
                            resource: RefEq::Borrowed(resource),
                        },
                    );
                    self.existing_borrows.insert(resource as *const R, id);
                    id
                }
            }
            RefEq::Owned(resource) => {
                let id = tracker.new_resource(R::RESOURCE_TYPE, None);
                self.resources.insert(
                    id,
                    RenderResourceMeta {
                        descriptor,
                        resource: RefEq::Owned(resource),
                    },
                );
                id
            }
        }
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
        todo!()
    }
}

impl<'g, R: DescribedRenderResource> IntoRenderResource<'g> for NewRenderResource<'g, R> {
    type Resource = R;

    fn into_render_resource(self, graph: &mut RenderGraphBuilder<'g>) -> RenderHandle<'g, R> {
        todo!()
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
    data: PhantomData<RenderGraphExecution<'g>>,
}

impl<'g> RenderDependencies<'g> {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    #[inline]
    pub fn read<R: RenderResource>(&mut self, resource: &RenderHandle<'g, R>) -> &mut Self {
        self.reads.insert(resource.id());
        self
    }

    #[inline]
    pub fn write<R: WriteRenderResource>(
        &mut self,
        resource: &mut RenderHandle<'g, R>,
    ) -> &mut Self {
        self.writes.insert(resource.id());
        self
    }

    #[inline]
    pub fn add(&mut self, dependencies: impl IntoRenderDependencies<'g>) -> &mut Self {
        dependencies.into_render_dependencies(self);
        self
    }

    #[inline]
    pub fn of(dependencies: impl IntoRenderDependencies<'g>) -> Self {
        let mut new = Self::default();
        new.add(dependencies);
        new
    }

    pub(super) fn iter_reads(&self) -> impl Iterator<Item = RenderResourceId> + '_ {
        self.reads.iter().copied()
    }

    pub(super) fn iter_writes(&self) -> impl Iterator<Item = RenderResourceId> + '_ {
        self.writes.iter().copied()
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = RenderResourceId> + '_ {
        self.iter_reads().chain(self.iter_writes())
    }
}

pub fn render_deps<'g>(dependencies: impl IntoRenderDependencies<'g>) -> RenderDependencies<'g> {
    RenderDependencies::of(dependencies)
}

impl<'g, R: RenderResource> IntoRenderDependencies<'g> for &RenderHandle<'g, R> {
    #[inline]
    fn into_render_dependencies(self, dependencies: &mut RenderDependencies<'g>) {
        dependencies.read(self);
    }
}

impl<'g, R: WriteRenderResource> IntoRenderDependencies<'g> for &mut RenderHandle<'g, R> {
    #[inline]
    fn into_render_dependencies(self, dependencies: &mut RenderDependencies<'g>) {
        dependencies.write(self);
    }
}

pub trait IntoRenderDependencies<'g> {
    fn into_render_dependencies(self, dependencies: &mut RenderDependencies<'g>);
}

macro_rules! impl_into_render_dependencies {
    ($(($T: ident, $t: ident)),*) => {
        #[allow(unused_variables)]
        impl <'g, $($T: IntoRenderDependencies<'g>),*> IntoRenderDependencies<'g> for ($($T,)*) {
            fn into_render_dependencies(self, dependencies: &mut RenderDependencies<'g>) {
                let ($($t,)*) = self;
                $($t.into_render_dependencies(dependencies);)*
            }
        }
    };
}

all_tuples!(impl_into_render_dependencies, 0, 16, T, t);
