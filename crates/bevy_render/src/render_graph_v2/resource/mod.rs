use std::{
    borrow::{Borrow, Cow},
    marker::PhantomData,
    ops::Index,
    sync::Arc,
};

use bevy_ecs::world::World;
use bevy_utils::{all_tuples, HashMap, HashSet};
use std::hash::Hash;

use crate::renderer::RenderDevice;

use super::{seal, NodeContext, RenderGraph};

pub mod bind_group;
pub mod buffer;
pub mod pipeline;
pub mod texture;

#[derive(Default)]
pub struct ResourceTracker<'g> {
    next_id: u32, //TODO: slotmap instead for better handling of resource clearing upon frame end
    generations: Vec<RenderResourceGenerationMeta<'g>>,
}

struct RenderResourceGenerationMeta<'g> {
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
        self.generations.push(RenderResourceGenerationMeta {
            generation: 0,
            dependencies,
        });
        RenderResourceId { id }
    }

    pub(super) fn write(&mut self, id: RenderResourceId) {
        // let meta = &mut self.generations[id.id as usize];
        // meta.generation += 1;
        // if let Some(deps) = &meta.dependencies {
        //     for id in deps.iter_writes() {
        //         self.write(id);
        //     }
        // }
        todo!()
    }

    pub(super) fn generation(&self, id: RenderResourceId) -> RenderResourceGeneration {
        // let meta = &self.generations[id.id as usize];
        // let mut total_generation = meta.generation;
        // if let Some(deps) = &meta.dependencies {
        //     for id in deps.iter() {
        //         total_generation += self.generation(id);
        //     }
        // }
        // total_generation
        todo!()
    }
}

pub trait RenderResource: seal::Super + 'static {
    type Descriptor: Send + Sync + 'static;
    type Data: Send + Sync + Clone + 'static;
    type Store<'g>: RenderStore<'g, Self>;
    // type PersistentStore: RenderStore<'static, Self>;

    fn get_store<'a, 'g: 'a>(graph: &'a RenderGraph<'g>, _: seal::Token) -> &'a Self::Store<'g>;
    fn get_store_mut<'a, 'g: 'a>(
        graph: &'a mut RenderGraph<'g>,
        _: seal::Token,
    ) -> &'a mut Self::Store<'g>;

    fn from_data<'a>(data: &'a Self::Data, world: &'a World) -> Option<&'a Self>;
    fn from_descriptor(
        descriptor: &Self::Descriptor,
        world: &World,
        render_device: &RenderDevice,
    ) -> Self::Data;
}

pub trait RenderStore<'g, R: RenderResource>: seal::Super {
    fn insert(
        &mut self,
        key: RenderResourceId,
        data: RenderResourceInit<'g, R>,
        world: &World,
        render_device: &RenderDevice,
    );

    fn get<'a: 'g>(
        &'a self,
        world: &'a World,
        key: RenderResourceId,
    ) -> Option<&'a RenderResourceMeta<'g, R>>;
}

// pub trait RetainedRenderStore<R: RenderResource>: WriteRenderStore<R> {
//     fn retain(&mut self, key: u16, label: InternedRenderLabel);
//
//     fn get_retained(&mut self, label: InternedRenderLabel) -> Option<RenderResourceMeta<R>>;
// }

pub trait WriteRenderResource: RenderResource {}

// pub trait RetainedRenderResource: WriteRenderResource
// where
//     <Self as RenderResource>::Store: RetainedRenderStore<Self>,
// {
// }

pub struct RenderResourceMeta<'g, R: RenderResource> {
    pub descriptor: Option<R::Descriptor>,
    pub resource: Cow<'g, R::Data>,
}

impl<'g, R: RenderResource> Clone for RenderResourceMeta<'g, R>
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

pub enum RenderResourceInit<'g, R: RenderResource> {
    FromDescriptor(R::Descriptor),
    Resource(RenderResourceMeta<'g, R>),
    DependentResource(
        RenderDependencies<'g>,
        RenderResourceMeta<'g, R>,
        seal::Token,
    ),
}

pub struct SimpleRenderStore<'g, R: RenderResource> {
    resources: HashMap<RenderResourceId, RenderResourceMeta<'g, R>>,
    // resources_to_retain: HashMap<RenderResourceId, InternedRenderLabel>,
    // retained_resources: HashMap<InternedRenderLabel, RenderResourceMeta<R>>,
}

impl<'g, R: RenderResource> seal::Super for SimpleRenderStore<'g, R> {}

impl<'g, R: RenderResource> RenderStore<'g, R> for SimpleRenderStore<'g, R> {
    fn insert(
        &mut self,
        id: RenderResourceId,
        data: RenderResourceInit<R>,
        world: &World,
        render_device: &RenderDevice,
    ) {
        // match data {
        //     RenderResourceInit::FromDescriptor(descriptor) => {
        //         let resource = R::from_descriptor(&descriptor, world, render_device);
        //         self.resources.insert(
        //             id,
        //             RenderResourceMeta::Owned {
        //                 descriptor: Some(descriptor),
        //                 resource,
        //             },
        //         );
        //     }
        //     RenderResourceInit::Resource(resource) => self.resources.insert(id, resource),
        // }
    }

    fn get<'a: 'g>(
        &'a self,
        _world: &'g World,
        id: RenderResourceId,
    ) -> Option<&'a RenderResourceMeta<'g, R>> {
        self.resources.get(&id)
    }
}

// impl<R: RenderResource> WriteRenderStore<R> for SimpleRenderStore<R> {
// fn get_mut<'a>(
//     &'a mut self,
//     world: &'a World,
//     key: u16,
// ) -> Option<&'a mut RenderResourceMeta<R>> {
//     self.resources.get_mut(&key)
// }
//
// fn take<'a>(&'a mut self, world: &'a World, key: u16) -> Option<RenderResourceMeta<R>> {
//     self.resources.remove(&key)
// }
//}

// impl<R: RenderResource> RetainedRenderStore<R> for SimpleRenderStore<R> {
//     fn retain(&mut self, key: u16, label: InternedRenderLabel) {
//         self.resources_to_retain.insert(key, label);
//     }
//
//     fn get_retained(&mut self, label: InternedRenderLabel) -> Option<RenderResourceMeta<R>> {
//         self.retained_resources.remove(&label)
//     }
// }

impl<'g, R: RenderResource> Default for SimpleRenderStore<'g, R> {
    fn default() -> Self {
        Self {
            resources: Default::default(),
            //queued_resources: Default::default(),
            // retained_resources: Default::default(),
            // resources_to_retain: Default::default(),
        }
    }
}
pub struct CachedRenderStore<'g, R: RenderResource>
where
    R::Descriptor: Clone + Hash + Eq,
{
    resources: HashMap<RenderResourceId, RenderResourceMeta<'g, R>>,
    //queued_resources: HashMap<RenderResourceId, DeferredResourceInit<R>>,
    //cached_resources: HashMap<R::Descriptor, RenderResourceMeta<'g, R>>, //TODO: switch to using
    //separate persistent store type
}

impl<'g, R: RenderResource> seal::Super for CachedRenderStore<'g, R> where
    R::Descriptor: Clone + Hash + Eq
{
}

impl<'g, R: RenderResource> RenderStore<'g, R> for CachedRenderStore<'g, R>
where
    R::Descriptor: Clone + Hash + Eq,
{
    fn insert(
        &mut self,
        id: RenderResourceId,
        data: RenderResourceInit<R>,
        world: &World,
        render_device: &RenderDevice,
    ) {
        // match data {
        //     RenderResourceInit::FromDescriptor(descriptor) => {
        //         let sampler = self
        //             .cached_resources
        //             .entry(descriptor.clone())
        //             .or_insert_with(|| {
        //                 let sampler = R::from_descriptor(&descriptor, world, render_device);
        //                 RenderResourceMeta {
        //                     descriptor: Some(descriptor),
        //                     resource: Cow::Owned(sampler),
        //                 }
        //             });
        //         self.resources.insert(id, sampler.clone());
        //     }
        //     RenderResourceInit::Resource(meta) => {
        //         if let Some(descriptor) = meta.descriptor {
        //             let meta = Arc::new(meta);
        //             self.cached_resources
        //                 .entry(descriptor)
        //                 .or_insert(meta.clone());
        //             self.resources.insert(id, meta);
        //         } else {
        //             self.resources.insert(id, Arc::new(meta));
        //         };
        //     }
        //     RenderResourceInit::Borrow(meta, data) => todo!(),
        // }
        todo!()
    }

    fn get<'a: 'g>(
        &'a self,
        world: &'a World,
        key: RenderResourceId,
    ) -> Option<&'a RenderResourceMeta<'g, R>> {
        todo!()
    }
}

impl<'g, R: RenderResource> Default for CachedRenderStore<'g, R>
where
    R::Descriptor: Clone + Hash + Eq,
{
    fn default() -> Self {
        Self {
            resources: Default::default(),
            // cached_resources: Default::default(),
        }
    }
}

pub trait IntoRenderResource<'g> {
    type Resource: RenderResource;

    fn into_render_resource(
        self,
        world: &World,
        render_device: &RenderDevice,
    ) -> RenderResourceInit<'g, Self::Resource>;
}

impl<'g, R: RenderResource<Data = R> + Clone, F: FnOnce(&RenderDevice) -> R> IntoRenderResource<'g>
    for F
{
    type Resource = R;

    fn into_render_resource(
        self,
        _world: &World,
        render_device: &RenderDevice,
    ) -> RenderResourceInit<'g, Self::Resource> {
        RenderResourceInit::Resource(RenderResourceMeta {
            descriptor: None,
            resource: Cow::Owned((self)(render_device)),
        })
    }
}

impl<'g, R: RenderResource> IntoRenderResource<'g> for RenderResourceInit<'g, R> {
    type Resource = R;

    fn into_render_resource(
        self,
        _world: &World,
        _render_device: &RenderDevice,
    ) -> RenderResourceInit<'g, R> {
        self
    }
}

impl<'g, R: RenderResource> IntoRenderResource<'g> for RenderResourceMeta<'g, R> {
    type Resource = R;

    fn into_render_resource(
        self,
        world: &World,
        render_device: &RenderDevice,
    ) -> RenderResourceInit<'g, Self::Resource> {
        RenderResourceInit::Resource(self)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct RenderResourceId {
    id: u32,
}

pub type RenderResourceGeneration = u16;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub(super) enum RenderResourceStorageType {
    Owned,
    Borrowed,
}

pub struct RenderHandle<'a, R: RenderResource> {
    id: RenderResourceId,
    // deps: DependencySet,
    data: PhantomData<&'a R>,
}

impl<'a, R: RenderResource> PartialEq for RenderHandle<'a, R> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id //&& self.deps == other.deps
    }
}

impl<'a, R: RenderResource> Eq for RenderHandle<'a, R> {}

impl<'a, R: RenderResource> Hash for RenderHandle<'a, R> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<'a, R: RenderResource> Copy for RenderHandle<'a, R> {}

impl<'a, R: RenderResource> Clone for RenderHandle<'a, R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, R: RenderResource> RenderHandle<'a, R> {
    pub(super) fn new(id: RenderResourceId) -> Self {
        Self {
            id,
            // deps: Default::default(),
            data: PhantomData,
        }
    }

    // pub(super) fn new_with_deps(id: RenderResourceId, deps: DependencySet) -> Self {
    //     Self {
    //         id,
    //         deps,
    //         data: PhantomData,
    //     }
    // }

    pub(super) fn id(&self) -> RenderResourceId {
        self.id
    }

    pub(super) fn generation(&self, graph: &RenderGraph) -> RenderResourceGeneration {
        graph.generation(self.id)
    }
}

#[derive(Default, PartialEq, Eq)]
pub struct RenderDependencies<'g> {
    reads: HashSet<RenderResourceId>,
    writes: HashSet<RenderResourceId>,
    data: PhantomData<RenderGraph<'g>>,
}

impl<'g> RenderDependencies<'g> {
    fn add(&mut self, dependency: impl Into<RenderDependency<'g>>) -> &mut Self {
        // let deps = resource.into_render_dependency(seal::Token);
        // for dep in deps {
        //     match dep.usage {
        //         RenderResourceUsage::Read => self.reads.insert(dep.id),
        //         RenderResourceUsage::Write => self.writes.insert(dep.id),
        //     }
        // }
        // RenderRef {
        //     id: dep.id,
        //     data: PhantomData,
        // }
        todo!()
    }

    fn of(dependencies: impl IntoRenderDependencies<'g>) -> Self {
        let mut new = Self::default();
        dependencies.into_render_dependencies(&mut new);
        new
    }

    pub(super) fn iter_reads<'a>(&'a self) -> impl Iterator<Item = RenderResourceId> + 'a {
        self.reads.iter().copied()
    }

    pub(super) fn iter_writes<'a>(&'a self) -> impl Iterator<Item = RenderResourceId> + 'a {
        self.writes.iter().copied()
    }

    pub(super) fn iter<'a>(&'a self) -> impl Iterator<Item = RenderResourceId> + 'a {
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

impl<'g, R: RenderResource> From<&mut RenderHandle<'g, R>> for RenderDependency<'g> {
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
        impl <'g, $($T: IntoRenderDependencies<'g>),*> IntoRenderDependencies<'g> for ($($T,)*) {
            fn into_render_dependencies(self, dependencies: &mut RenderDependencies<'g>) {
                let ($($t,)*) = self;
                $($t.into_render_dependencies(dependencies);)*
            }
        }
    };
}

all_tuples!(impl_into_render_dependencies, 0, 16, T, t);
