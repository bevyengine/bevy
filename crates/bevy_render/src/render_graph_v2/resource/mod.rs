use std::{marker::PhantomData, sync::Arc};

use bevy_ecs::world::World;
use bevy_utils::{all_tuples, HashMap, HashSet};
use bitflags::iter::IterNames;
use std::hash::Hash;
use wgpu::CommandEncoder;

use crate::{
    render_graph::InternedRenderLabel,
    render_resource::{BindGroup, Texture},
    renderer::RenderDevice,
};

use bind_group::RenderBindGroup;

use self::bind_group::UnsafeRenderBindGroup;

use super::RenderGraph;

pub mod bind_group;
pub mod buffer;
pub mod pipeline;
pub mod texture;

pub trait RenderResource: Sized + Send + Sync + 'static {
    type Descriptor: Send + Sync + 'static;
    type Data: Send + Sync + 'static;
    type Store: RenderStore<Self>;

    fn get_store(graph: &RenderGraph) -> &Self::Store; //todo: proper generic resource table & make sure external users can't call this function
    fn get_store_mut(graph: &mut RenderGraph) -> &mut Self::Store;

    fn from_data<'a>(data: &'a Self::Data, world: &'a World) -> Option<&'a Self>;

    fn from_descriptor(
        descriptor: &Self::Descriptor,
        world: &World,
        render_device: &RenderDevice,
    ) -> Self::Data;
}

pub trait RenderStore<R: RenderResource>: Send + Sync + 'static {
    fn insert(
        &mut self,
        key: u16,
        data: RenderResourceInit<R>,
        world: &World,
        render_device: &RenderDevice,
    );

    fn get<'a>(&'a self, world: &'a World, key: u16) -> Option<&'a RenderResourceMeta<R>>;

    fn get_mut<'a>(
        &'a mut self,
        world: &'a World,
        key: u16,
    ) -> Option<&'a mut RenderResourceMeta<R>>;

    fn take<'a>(&'a mut self, world: &'a World, key: u16) -> Option<RenderResourceMeta<R>>;

    fn init_queued_resources(&mut self, world: &mut World, device: &RenderDevice);
}

pub trait RetainedRenderStore<R: RenderResource>: RenderStore<R> {
    fn retain(&mut self, key: u16, label: InternedRenderLabel);

    fn get_retained(&mut self, label: InternedRenderLabel) -> Option<RenderResourceMeta<R>>;
}

pub trait WriteRenderResource: RenderResource {}

pub trait RetainedRenderResource: WriteRenderResource
where
    <Self as RenderResource>::Store: RetainedRenderStore<Self>,
{
}

#[derive(Clone)]
pub struct RenderResourceMeta<R: RenderResource> {
    pub(super) descriptor: Option<R::Descriptor>,
    pub(super) resource: R::Data,
}

type DeferredResourceInit<R> =
    Box<dyn FnOnce(&mut World, &RenderDevice) -> RenderResourceMeta<R> + Send + Sync + 'static>;

pub enum RenderResourceInit<R: RenderResource> {
    FromDescriptor(R::Descriptor),
    Eager(RenderResourceMeta<R>),
    Deferred(DeferredResourceInit<R>),
}

pub struct SimpleRenderStore<R: RenderResource> {
    resources: HashMap<u16, RenderResourceMeta<R>>,
    queued_resources: HashMap<u16, DeferredResourceInit<R>>,
    resources_to_retain: HashMap<u16, InternedRenderLabel>,
    retained_resources: HashMap<InternedRenderLabel, RenderResourceMeta<R>>,
}

impl<R: RenderResource> RenderStore<R> for SimpleRenderStore<R> {
    fn insert(
        &mut self,
        key: u16,
        data: RenderResourceInit<R>,
        world: &World,
        render_device: &RenderDevice,
    ) {
        match data {
            RenderResourceInit::FromDescriptor(descriptor) => {
                let resource = R::from_descriptor(&descriptor, world, render_device);
                self.resources.insert(
                    key,
                    RenderResourceMeta {
                        descriptor: Some(descriptor),
                        resource,
                    },
                );
            }
            RenderResourceInit::Eager(meta) => {
                self.resources.insert(key, meta);
            }
            RenderResourceInit::Deferred(init) => {
                self.queued_resources.insert(key, init);
            }
        }
    }

    fn get<'a>(&'a self, _world: &'a World, key: u16) -> Option<&'a RenderResourceMeta<R>> {
        self.resources.get(&key)
    }

    fn get_mut<'a>(
        &'a mut self,
        world: &'a World,
        key: u16,
    ) -> Option<&'a mut RenderResourceMeta<R>> {
        self.resources.get_mut(&key)
    }

    fn take<'a>(&'a mut self, world: &'a World, key: u16) -> Option<RenderResourceMeta<R>> {
        self.resources.remove(&key)
    }

    fn init_queued_resources(&mut self, world: &mut World, device: &RenderDevice) {
        for (key, init) in self.queued_resources.drain() {
            self.resources.insert(key, (init)(world, device));
        }
    }
}

pub struct CachedRenderStore<R: RenderResource>
where
    R::Descriptor: Clone + Hash + Eq,
{
    resources: HashMap<u16, Arc<RenderResourceMeta<R>>>,
    queued_resources: HashMap<u16, DeferredResourceInit<R>>,
    cached_resources: HashMap<R::Descriptor, Arc<RenderResourceMeta<R>>>,
}

impl<R: RenderResource> RenderStore<R> for CachedRenderStore<R>
where
    R::Descriptor: Clone + Hash + Eq,
{
    fn insert(
        &mut self,
        key: u16,
        data: RenderResourceInit<R>,
        world: &World,
        render_device: &RenderDevice,
    ) {
        match data {
            RenderResourceInit::FromDescriptor(descriptor) => {
                let sampler = self
                    .cached_resources
                    .entry(descriptor.clone())
                    .or_insert_with(|| {
                        let sampler = R::from_descriptor(&descriptor, world, render_device);
                        Arc::new(RenderResourceMeta {
                            descriptor: Some(descriptor),
                            resource: sampler,
                        })
                    });
                self.resources.insert(key, sampler.clone());
            }
            RenderResourceInit::Eager(meta) => {
                if let Some(descriptor) = meta.descriptor.clone() {
                    let meta = Arc::new(meta);
                    self.cached_resources
                        .entry(descriptor)
                        .or_insert(meta.clone());
                    self.resources.insert(key, meta);
                } else {
                    self.resources.insert(key, Arc::new(meta));
                };
            }
            RenderResourceInit::Deferred(init) => {
                self.queued_resources.insert(key, init);
            }
        }
    }

    fn get<'a>(&'a self, world: &'a World, key: u16) -> Option<&'a RenderResourceMeta<R>> {
        self.resources.get(&key).map(|meta| &**meta)
    }

    fn get_mut<'a>(
        &'a mut self,
        world: &'a World,
        key: u16,
    ) -> Option<&'a mut RenderResourceMeta<R>> {
        self.resources.get_mut(&key).map(|meta| &mut **meta)
    }

    fn take<'a>(&'a mut self, world: &'a World, key: u16) -> Option<RenderResourceMeta<R>> {
        self.resources.remove(&key).map(Arc::into_inner)
    }

    fn init_queued_resources(&mut self, world: &mut World, device: &RenderDevice) {
        for (key, init) in self.queued_resources.drain() {
            let meta = (init)(world, device);
            if let Some(descriptor) = meta.descriptor.clone() {
                self.cached_resources
                    .entry(descriptor)
                    .or_insert(Arc::new(meta));
            } else {
                self.resources.insert(key, Arc::new(meta));
            }
        }
    }
}

impl<R: RenderResource> RetainedRenderStore<R> for SimpleRenderStore<R> {
    fn retain(&mut self, key: u16, label: InternedRenderLabel) {
        self.resources_to_retain.insert(key, label);
    }

    fn get_retained(&mut self, label: InternedRenderLabel) -> Option<RenderResourceMeta<R>> {
        self.retained_resources.remove(&label)
    }
}

impl<R: RenderResource> Default for SimpleRenderStore<R> {
    fn default() -> Self {
        Self {
            retained_resources: Default::default(),
            resources: Default::default(),
            queued_resources: Default::default(),
            resources_to_retain: Default::default(),
        }
    }
}

impl<R: RenderResource> Default for CachedRenderStore<R>
where
    R::Descriptor: Clone + Hash + Eq,
{
    fn default() -> Self {
        Self {
            resources: Default::default(),
            queued_resources: Default::default(),
            cached_resources: Default::default(),
        }
    }
}

pub trait IntoRenderResource {
    type Resource: RenderResource;

    fn into_render_resource(
        self,
        world: &World,
        render_device: &RenderDevice,
    ) -> RenderResourceInit<Self::Resource>;
}

impl<R: RenderResource<Data = R>, F: FnOnce(&RenderDevice) -> R> IntoRenderResource for F {
    type Resource = R;

    fn into_render_resource(
        self,
        _world: &World,
        render_device: &RenderDevice,
    ) -> RenderResourceInit<Self::Resource> {
        RenderResourceInit::Eager(RenderResourceMeta {
            descriptor: None,
            resource: (self)(render_device),
        })
    }
}

#[derive(PartialEq, Eq, Hash)]
pub struct RenderHandle<'a, R: RenderResource> {
    id: RenderResourceId,
    data: PhantomData<&'a R>,
}

impl<'a, R: RenderResource> RenderHandle<'a, R> {
    pub(super) fn new(index: u16) -> Self {
        Self {
            id: RenderResourceId {
                index,
                generation: 0,
            },
            data: PhantomData,
        }
    }

    pub(super) fn index(&self) -> u16 {
        self.id.index
    }

    pub fn is_fresh(&self) -> bool {
        self.id.generation == 0
    }

    fn as_unsafe(&self) -> UnsafeRenderHandle<R> {
        UnsafeRenderHandle {
            id: self.id,
            usage_type: ResourceUsageType::Read,
            data: PhantomData,
        }
    }

    fn as_unsafe_mut(&mut self) -> UnsafeRenderHandle<R> {
        UnsafeRenderHandle {
            id: self.id,
            usage_type: ResourceUsageType::Write,
            data: PhantomData,
        }
    }

    fn into_unsafe(self) -> UnsafeRenderHandle<R> {
        UnsafeRenderHandle {
            id: self.id,
            usage_type: ResourceUsageType::Take,
            data: PhantomData,
        }
    }
}

pub enum RenderDependency {
    Read(RenderResourceId),
    ReadWrite(RenderResourceId),
    BindGroup(UnsafeRenderBindGroup),
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct RenderResourceId {
    pub(super) index: u16,
    pub(super) generation: u16,
}

#[derive(Default)]
pub struct DependencySet {
    reads: HashSet<RenderResourceId>,
    writes: HashSet<RenderResourceId>,
    takes: HashSet<RenderResourceId>,
    bind_groups: HashSet<UnsafeRenderBindGroup>,
}

impl DependencySet {
    pub fn add<R: RenderResource>(&mut self, resource: &UnsafeRenderHandle<R>) {
        match resource.usage_type {
            ResourceUsageType::Read => {
                self.reads.insert(resource.id);
            }
            ResourceUsageType::Write => {
                self.writes.insert(resource.id);
            }
            ResourceUsageType::Take => {
                self.takes.insert(resource.id);
            }
        }
    }

    pub fn add_bind_group(&mut self, bind_group: &RenderBindGroup) {
        self.bind_groups.insert(bind_group);
    }
}

#[derive(Copy, Clone)]
enum ResourceUsageType {
    Read,
    Write,
    Take,
}

pub struct UnsafeRenderHandle<R: RenderResource> {
    id: RenderResourceId,
    usage_type: ResourceUsageType,
    data: PhantomData<R>,
}

impl<R: RenderResource> UnsafeRenderHandle<R> {
    pub fn get<'w>(&self, graph: &'w RenderGraph, world: &'w World) -> Option<&'w R> {
        todo!()
    }

    pub fn get_mut<'w>(&self, graph: &'w mut RenderGraph, world: &'w World) -> Option<&'w mut R> {}

    pub fn take<'w>(&self, graph: &'w mut RenderGraph, world: &'w World) -> Option<R> {
        todo!()
    }
}

impl UnsafeRenderBindGroup {
    fn new(bind_group: RenderBindGroup) -> Self {
        Self { id: bind_group.id }
    }

    fn get(graph: &RenderGraph) -> Option<&BindGroup> {
        todo!()
    }
}

impl<R: RenderResource> Copy for UnsafeRenderHandle<R> {}
impl<R: RenderResource> Clone for UnsafeRenderHandle<R> {
    fn clone(&self) -> Self {
        *self
    }
}

pub trait RenderData {
    type Item<'w>;
    type Handle: Send + Sync + 'static;

    fn add_dependencies(handle: &Self::Handle, dependencies: &mut DependencySet);
    fn get_from_graph<'w>(
        fetch: &Self::Handle,
        graph: &'w mut RenderGraph,
        world: &'w World,
    ) -> Option<Self::Item<'w>>;
}

pub trait IntoRenderData {
    type Data: RenderData;

    fn into_render_data(self) -> <Self::Data as RenderData>::Handle;
}

impl<R: RenderResource> RenderData for &R {
    type Item<'w> = &'w R;
    type Handle = UnsafeRenderHandle<R>;

    fn add_dependencies(handle: &Self::Handle, dependencies: &mut DependencySet) {
        dependencies.add(handle)
    }

    fn get_from_graph<'w>(
        handle: &Self::Handle,
        graph: &'w mut RenderGraph,
        world: &'w World,
    ) -> Option<Self::Item<'w>> {
        R::get_store(graph)
            .get(world, handle.id)
            .and_then(|meta| R::from_data(&meta.resource, world))
    }
}

impl<'a, R: RenderResource> IntoRenderData for &RenderHandle<'a, R> {
    type Data = &'a R;

    fn into_render_data(self) -> <Self::Data as RenderData>::Handle {
        self.as_unsafe()
    }
}

impl<'a, R: RenderResource> IntoRenderData for &mut RenderHandle<'a, R> {
    type Data = &'a mut R;

    fn into_render_data(self) -> <Self::Data as RenderData>::Handle {
        self.as_unsafe_mut()
    }
}

impl<'a, R: RenderResource> IntoRenderData for RenderHandle<'a, R> {
    type Data = R;

    fn into_render_data(self) -> <Self::Data as RenderData>::Handle {
        self.into_unsafe()
    }
}

impl<'a> IntoRenderData for &RenderBindGroup<'a> {
    type Data = &'a BindGroup;

    fn into_render_data(self) -> <Self::Data as RenderData>::Handle {
        self.as_unsafe()
    }
}

macro_rules! impl_into_render_data {
    ($(($T: ident, $t: ident)),*) => {
        impl <$($T: IntoRenderData),*> IntoRenderData for ($($T,)*) {
            type Data = ($($T::Data,)*);

            #[allow(clippy::unused_unit)]
            fn into_render_data(self) -> <Self::Data as RenderData>::Handle {
                let ($($t,)*) = self;
                ($($t.into_render_data(),)*)
            }
        }
    }
}

all_tuples!(impl_into_render_data, 0, 16, T, t);

impl<R: RenderResource> RenderData for &R {
    type Item<'w> = &'w R;
    type Handle = UnsafeRenderHandle<R>;

    fn add_dependencies(handle: &Self::Handle, dependencies: &mut DependencySet) {
        dependencies.add(handle)
    }

    fn get_from_graph<'w>(
        handle: &Self::Handle,
        graph: &'w mut RenderGraph,
        world: &'w World,
    ) -> Option<Self::Item<'w>> {
        handle.get(graph, world)
    }
}

impl<R: RenderResource> RenderData for &mut R {
    type Item<'w> = &'w mut R;
    type Handle = UnsafeRenderHandle<R>;

    fn add_dependencies(handle: &Self::Handle, dependencies: &mut DependencySet) {
        dependencies.add(handle)
    }

    fn get_from_graph<'w>(
        handle: &Self::Handle,
        graph: &'w mut RenderGraph,
        world: &'w World,
    ) -> Option<Self::Item<'w>> {
        handle.get_mut(graph, world)
    }
}

impl<R: RenderResource> RenderData for R {
    type Item<'w> = R;
    type Handle = UnsafeRenderHandle<R>;

    fn add_dependencies(handle: &Self::Handle, dependencies: &mut DependencySet) {
        dependencies.add(handle)
    }

    fn get_from_graph<'w>(
        handle: &Self::Handle,
        graph: &'w mut RenderGraph,
        world: &'w World,
    ) -> Option<Self::Item<'w>> {
        handle.take(graph, world)
    }
}

impl RenderData for &BindGroup {
    type Item<'w> = &'w BindGroup;
    type Handle = UnsafeRenderBindGroup;

    fn add_dependencies(handle: &Self::Handle, dependencies: &mut DependencySet) {
        dependencies.add_bind_group(handle);
    }

    fn get_from_graph<'w>(
        handle: &Self::Handle,
        graph: &'w mut RenderGraph,
        world: &'w World,
    ) -> Option<Self::Item<'w>> {
    }
}

impl<D: RenderData, const N: usize> RenderData for [D; N] {
    type Item<'w> = [D::Item<'w>; N];
    type Handle = [D::Handle; N];

    fn add_dependencies(fetch: &Self::Handle, dependencies: &mut DependencySet) {
        for f in fetch {
            D::add_dependencies(f, dependencies);
        }
    }

    fn get_from_graph<'w>(
        handle: &Self::Handle,
        graph: &'w mut RenderGraph,
        world: &'w World,
    ) -> Option<Self::Item<'w>> {
        let mut items: Self::Item<'w> = [];
        for i in 0..N {
            items[i] = D::get_from_graph(&handle[i], graph, world)?;
        }
        Some(items)
    }
}

macro_rules! impl_render_data {
    ($(($T: ident, $t: ident, $i: ident)),*) => {
        impl <$($T: RenderData),*> RenderData for ($($T,)*) {
            type Item<'w> = ($($T::Item<'w>,)*);
            type Handle = ($($T::Handle,)*);

            fn add_dependencies(($($t,)*): &Self::Handle, dependencies: &mut DependencySet) {
                $($T::add_dependencies($t, dependencies);)*
            }

            fn get_from_graph<'w>(
                ($($t,)*): &Self::Handle,
                graph: &'w mut RenderGraph,
                world: &'w World,
            ) -> Option<Self::Item<'w>> {
                match ($($T::get_from_graph($t, graph, world),)*) {
                    ($(Some($i),)*) => Some(($($i,)*)),
                    _ => None,
                }
            }
        }
    };
}

all_tuples!(impl_render_data, 0, 16, T, t, i);

// impl<'a, R: RenderResource> IntoRenderDependency<'a> for &'a RenderHandle<R> {
//     fn into_render_dependency(self) -> RenderDependency {
//         RenderDependency::Read(self.id)
//     }
// }
//
// impl<'a, R: WriteRenderResource> IntoRenderDependency<'a> for &'a mut RenderHandle<R> {
//     fn into_render_dependency(self) -> RenderDependency {
//         let dep = RenderDependency::ReadWrite(self.id);
//         self.id.generation += 1;
//         dep
//     }
// }
//
// impl<'a> IntoRenderDependency<'a> for &'a RenderBindGroup {
//     fn into_render_dependency(self) -> RenderDependency {
//         RenderDependency::BindGroup(*self)
//     }
// }
//
// pub trait IntoRenderDependencies {
//     fn into_render_dependencies(self) -> impl Iterator<Item = RenderDependency>;
// }
//
// impl<'a, T: IntoRenderDependency<'a>> IntoRenderDependencies<'a> for T {
//     fn into_render_dependencies(self) -> impl Iterator<Item = RenderDependency> {
//         vec![self.into_render_dependency()].into_iter()
//     }
// }
//
// macro_rules! impl_into_render_resource_ids {
//     ($(($T: ident, $t: ident)),*) => {
//         impl <'a, $($T: IntoRenderDependency<'a>),*> IntoRenderDependencies<'a> for ($($T,)*) {
//             fn into_render_dependencies(self) -> impl Iterator<Item = RenderDependency> {
//                 let ($($t,)*) = self;
//                 vec![$($t.into_render_dependency()),*].into_iter()
//             }
//         }
//     };
// }
//
// all_tuples!(impl_into_render_resource_ids, 0, 16, T, t);
