use std::{
    borrow::{Borrow, Cow},
    collections::VecDeque,
    fmt::Debug,
    marker::PhantomData,
};

use bevy_ecs::world::World;
use bevy_utils::{HashMap, HashSet};
use std::hash::Hash;

use bevy_render::{render_resource::PipelineCache, renderer::RenderDevice};

use super::{NodeContext, RenderGraph, RenderGraphBuilder};

pub mod bind_group;
pub mod buffer;
pub mod pipeline;
pub mod texture;

#[derive(Default)]
pub struct ResourceTracker<'g> {
    next_id: u32,
    resources: Vec<ResourceInfo<'g>>,
}

#[derive(PartialEq, Eq, Debug)]
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

        if dependencies.as_ref().is_some_and(|deps| {
            deps.iter()
                .any(|id| self.resources[id.id as usize].resource_type == resource_type)
        }) {
            panic!("Resources cannot directly depend on a resource of the same type");
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
    fn collect_many_dependencies(
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

pub trait RenderResource: Sized + Send + Sync + Clone + Hash + Eq + 'static {
    const RESOURCE_TYPE: ResourceType;
    type Meta<'g>: Clone + IntoRenderResource<'g, Resource = Self>; //todo: reformat to trait method?

    fn import_resource<'g>(
        graph: &mut RenderGraphBuilder<'_, 'g>,
        meta: Self::Meta<'g>,
        resource: Cow<'g, Self>,
    ) -> RenderHandle<'g, Self>;

    // fn new_resource<'g>(
    //     graph: &mut RenderGraphBuilder<'_, 'g>,
    //     meta: Self::Meta<'g>,
    // ) -> RenderHandle<'g, Self>;

    // fn new<'g>(graph: &mut RenderGraphBuilder<'g>, meta: Self::Meta<'g>) -> RenderHandle<'g, Self>;

    fn get<'n, 'g: 'n>(
        context: &'n NodeContext<'n, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'n Self>;

    fn get_meta<'a, 'b: 'a, 'g: 'b>(
        graph: &'a RenderGraphBuilder<'b, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a Self::Meta<'g>>;
}

pub trait WriteRenderResource: RenderResource {}

pub trait CacheRenderResource: RenderResource {
    type Key: Clone + Hash + Eq + 'static;

    fn key_from_meta<'a, 'g: 'a>(meta: &'a Self::Meta<'g>) -> &'a Self::Key;
}

pub trait UsagesRenderResource: RenderResource {
    type Usages: Send + Sync + Debug + 'static;

    fn get_meta_mut<'a, 'b: 'a, 'g: 'b>(
        graph: &'a mut RenderGraphBuilder<'b, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a mut Self::Meta<'g>>;

    fn has_usages(meta: &Self::Meta<'_>, usages: &Self::Usages) -> bool;
    fn add_usages(meta: &mut Self::Meta<'_>, usages: Self::Usages);
}

struct RenderResourceMeta<'g, R: RenderResource> {
    pub meta: R::Meta<'g>,
    pub resource: Cow<'g, R>,
}

impl<'g, R: RenderResource + Clone> Clone for RenderResourceMeta<'g, R>
where
    R::Meta<'g>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            meta: self.meta.clone(),
            resource: self.resource.clone(),
        }
    }
}

#[allow(clippy::type_complexity)]
pub(super) struct RenderResources<'g, R: RenderResource> {
    resources: HashMap<RenderResourceId, RenderResourceMeta<'g, R>>,
    existing_resources: HashMap<Cow<'g, R>, RenderResourceId>,
    queued_resources: HashMap<RenderResourceId, R::Meta<'g>>,
}

impl<'g, R: RenderResource> Default for RenderResources<'g, R> {
    fn default() -> Self {
        Self {
            resources: Default::default(),
            existing_resources: Default::default(),
            queued_resources: Default::default(),
        }
    }
}

impl<'g, R: RenderResource> RenderResources<'g, R> {
    pub fn import_resource(
        &mut self,
        tracker: &mut ResourceTracker<'g>,
        dependencies: Option<RenderDependencies<'g>>,
        meta: R::Meta<'g>,
        resource: Cow<'g, R>,
    ) -> RenderResourceId {
        self.existing_resources
            .get(&resource)
            .copied()
            .unwrap_or_else(|| {
                let id = tracker.new_resource(R::RESOURCE_TYPE, dependencies);
                self.existing_resources.insert(resource.clone(), id);
                self.resources
                    .insert(id, RenderResourceMeta { meta, resource });
                id
            })
    }

    pub fn new_resource(
        &mut self,
        tracker: &mut ResourceTracker<'g>,
        dependencies: Option<RenderDependencies<'g>>,
        meta: R::Meta<'g>,
    ) -> RenderResourceId {
        let id = tracker.new_resource(R::RESOURCE_TYPE, dependencies);
        self.queued_resources.insert(id, meta);
        id
    }

    pub fn create_queued_resources<
        F: FnMut(&World, &RenderDevice, &RenderGraph<'g>, &R::Meta<'g>) -> R,
    >(
        &mut self,
        world: &World,
        render_device: &RenderDevice,
        render_graph: &RenderGraph<'g>,
        mut f: F,
    ) {
        for (id, meta) in self.queued_resources.drain() {
            let resource = (f)(world, render_device, render_graph, &meta);
            self.resources.insert(
                id,
                RenderResourceMeta {
                    meta,
                    resource: Cow::Owned(resource),
                },
            );
        }
    }

    pub fn get_meta(&self, id: RenderResourceId) -> Option<&R::Meta<'g>> {
        let check_normal = self.resources.get(&id).map(|res| &res.meta);
        let check_queued = self.queued_resources.get(&id);
        check_normal.or(check_queued)
    }

    pub fn get_meta_mut(&mut self, id: RenderResourceId) -> Option<&mut R::Meta<'g>> {
        self.queued_resources.get_mut(&id)
    }

    pub fn get(&self, id: RenderResourceId) -> Option<&R> {
        self.resources.get(&id).map(|meta| meta.resource.borrow())
    }
}

impl<'g, R: CacheRenderResource> RenderResources<'g, R> {
    pub fn create_queued_resources_cached<
        F: FnMut(&World, &RenderDevice, &RenderGraph<'g>, &R::Meta<'g>) -> R,
    >(
        &mut self,
        cache: &mut CachedResources<R>,
        world: &World,
        render_device: &RenderDevice,
        render_graph: &RenderGraph<'g>,
        mut f: F,
    ) {
        for (_, meta) in &self.queued_resources {
            cache
                .cached_resources
                .entry(R::key_from_meta(meta).clone())
                .or_insert_with(|| (f)(world, render_device, render_graph, meta));
        }
    }

    pub fn borrow_cached_resources(&mut self, cache: &'g CachedResources<R>) {
        for (id, meta) in self.queued_resources.drain() {
            if let Some(resource) = cache.cached_resources.get(R::key_from_meta(&meta)) {
                self.resources.insert(
                    id,
                    RenderResourceMeta {
                        meta,
                        resource: Cow::Borrowed(resource),
                    },
                );
            }
        }
    }
}

pub struct CachedResources<R: CacheRenderResource> {
    cached_resources: HashMap<R::Key, R>,
}

impl<R: CacheRenderResource> Default for CachedResources<R> {
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
        graph: &mut RenderGraphBuilder<'_, 'g>,
    ) -> RenderHandle<'g, Self::Resource>;
}

// impl<'g, R: RenderResource, F: FnOnce(&RenderDevice) -> R> IntoRenderResource<'g> for F {
//     type Resource = R;
//
//     fn into_render_resource(
//         self,
//         graph: &mut RenderGraphBuilder<'g>,
//     ) -> RenderHandle<'g, Self::Resource> {
//         let resource = (self)(graph.render_device);
//         graph.into_resource(resource)
//     }
// }

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
    data: PhantomData<&'g ()>,
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
            let mut dependencies = $crate::core::resource::RenderDependencies::new();
            $crate::core::resource::extend_deps!(dependencies, $($dep),*);
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
