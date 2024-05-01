use std::{
    borrow::{Borrow, Cow},
    collections::VecDeque,
    fmt::Debug,
    marker::PhantomData,
};

use bevy_ecs::world::World;
use bevy_utils::{all_tuples, HashMap, HashSet};
use std::hash::Hash;

use crate::{
    render_resource::{
        BindGroup, BindGroupLayout, ComputePipeline, RenderPipeline, Sampler, Texture, TextureView,
    },
    renderer::RenderDevice,
};

use self::ref_eq::RefEq;

use super::{NodeContext, RenderGraph, RenderGraphBuilder};

pub mod bind_group;
pub mod buffer;
pub mod pipeline;
pub(crate) mod ref_eq;
pub mod texture;

#[derive(Default)]
pub struct ResourceTracker<'g> {
    next_id: u32,
    generations: Vec<GenerationInfo<'g>>,
}

struct GenerationInfo<'g> {
    generation: RenderResourceGeneration,
    dependencies: Option<RenderDependencies<'g>>,
}

impl<'g> ResourceTracker<'g> {
    pub(super) fn clear(&mut self) {
        self.next_id = 0;
        self.generations.clear();
    }

    pub(super) fn new_resource(
        &mut self,
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
        self.generations.push(GenerationInfo {
            generation: 0,
            dependencies,
        });
        RenderResourceId { id }
    }

    pub(super) fn write(&mut self, id: RenderResourceId) {
        self.collect_dependencies(id)
            .writes
            .into_iter()
            .for_each(|id| self.generations[id.id as usize].generation += 1);
    }

    pub(super) fn generation(&self, id: RenderResourceId) -> RenderResourceGeneration {
        self.collect_dependencies(id)
            .iter()
            .map(|id| self.generations[id.id as usize].generation)
            .sum()
    }

    fn collect_dependencies(&self, id: RenderResourceId) -> RenderDependencies<'g> {
        let mut dependencies = self.generations[id.id as usize]
            .dependencies
            .clone()
            .unwrap_or(Default::default());
        //TODO: THIS IS IMPORTANT
        todo!();
        dependencies
    }
}

pub trait RenderResource: Sized + Send + Sync + 'static {
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
        descriptor: Option<Self::Descriptor>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self>;

    fn get_descriptor<'g>(
        graph: &RenderGraph<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'g Self::Descriptor>;
}

pub trait UsagesRenderResource: DescribedRenderResource {
    type Usages: Send + Sync + Debug + 'static;

    fn get_descriptor_mut<'a, 'g: 'a>(
        graph: &'a mut RenderGraph<'g>,
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
    resource_factory: &'g dyn Fn(&RenderDevice, &R::Descriptor) -> R,
}

impl<'g, R: DescribedRenderResource> RenderResources<'g, R> {
    pub fn new<F: Fn(&RenderDevice, &R::Descriptor) -> R>(factory: &'g F) -> Self {
        Self {
            resources: Default::default(),
            existing_borrows: Default::default(),
            queued_resources: Default::default(),
            resource_factory: factory,
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
                    let id = tracker.new_resource(None);
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
                let id = tracker.new_resource(None);
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
        let id = tracker.new_resource(None);
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
        cache: &'g mut CachedResources<R>,
        render_device: &RenderDevice,
    ) where
        R::Descriptor: Clone + Hash + Eq,
    {
        let mut resources_to_borrow = Vec::new();
        for (id, descriptor) in self.queued_resources.drain() {
            cache
                .cached_resources
                .entry(descriptor.clone())
                .or_insert_with(|| (self.resource_factory)(render_device, &descriptor));
            resources_to_borrow.push((id, descriptor));
        }
        for (id, descriptor) in resources_to_borrow {
            let resource = cache.cached_resources.get(&descriptor).unwrap();
            self.resources.insert(
                id,
                RenderResourceMeta {
                    descriptor: Some(descriptor),
                    resource: RefEq::Borrowed(resource),
                },
            );
        }
    }

    pub fn get_descriptor(&self, id: RenderResourceId) -> Option<&R::Descriptor> {
        self.resources
            .get(&id)
            .and_then(|meta| meta.descriptor.as_ref())
    }

    pub fn get(&self, id: RenderResourceId) -> Option<&R> {
        self.resources.get(&id).map(|meta| meta.resource.borrow())
    }

    pub fn drain_to_cache(self, cache: &mut CachedResources<R>)
    where
        R::Descriptor: Clone + Hash + Eq,
    {
        for (_, meta) in self.resources {
            if let RenderResourceMeta {
                descriptor: Some(descriptor),
                resource: RefEq::Owned(resource),
            } = meta
            {
                cache.cached_resources.insert(descriptor, resource);
            }
        }
    }
}

impl<'g, R: UsagesRenderResource> RenderResources<'g, R> {
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
    // deps: DependencySet,
    data: PhantomData<&'g R>,
}

impl<'g, R: RenderResource> PartialEq for RenderHandle<'g, R> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id //&& self.deps == other.deps
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

    pub(super) fn generation(&self, graph: &RenderGraph) -> RenderResourceGeneration {
        graph.generation(self.id)
    }
}

#[derive(Default, PartialEq, Eq, Clone)]
pub struct RenderDependencies<'g> {
    reads: HashSet<RenderResourceId>,
    writes: HashSet<RenderResourceId>,
    data: PhantomData<RenderGraph<'g>>,
}

impl<'g> RenderDependencies<'g> {
    pub fn add(&mut self, dependency: impl Into<RenderDependency<'g>>) -> &mut Self {
        let dep: RenderDependency = dependency.into();
        match dep.usage {
            RenderResourceUsage::Read => {
                self.reads.insert(dep.id);
            }
            RenderResourceUsage::Write => {
                self.writes.insert(dep.id);
            }
        }
        self
    }

    fn of(dependencies: impl IntoRenderDependencies<'g>) -> Self {
        let mut new = Self::default();
        dependencies.into_render_dependencies(&mut new);
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

struct RenderDependency<'g> {
    id: RenderResourceId,
    usage: RenderResourceUsage,
    data: PhantomData<RenderGraph<'g>>,
}

#[derive(Copy, Clone)]
enum RenderResourceUsage {
    Read,
    Write,
}

impl<'g, R: RenderResource> From<&RenderHandle<'g, R>> for RenderDependency<'g> {
    fn from(value: &RenderHandle<'g, R>) -> Self {
        RenderDependency {
            id: value.id(),
            usage: RenderResourceUsage::Read,
            data: PhantomData,
        }
    }
}

impl<'g, R: WriteRenderResource> From<&mut RenderHandle<'g, R>> for RenderDependency<'g> {
    fn from(value: &mut RenderHandle<'g, R>) -> Self {
        RenderDependency {
            id: value.id(),
            usage: RenderResourceUsage::Write,
            data: PhantomData,
        }
    }
}

pub trait IntoRenderDependencies<'g> {
    fn into_render_dependencies(self, dependencies: &mut RenderDependencies<'g>);
}

impl<'g, T: Into<RenderDependency<'g>>> IntoRenderDependencies<'g> for T {
    fn into_render_dependencies(self, dependencies: &mut RenderDependencies<'g>) {
        dependencies.add(self);
    }
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
