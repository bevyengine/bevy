use std::{borrow::Borrow, marker::PhantomData, ops::Index, sync::Arc};

use bevy_ecs::world::World;
use bevy_utils::{HashMap, HashSet};
use std::hash::Hash;

use crate::renderer::RenderDevice;

use super::{seal, RenderGraph, RenderResourceGeneration};

pub mod bind_group;
pub mod buffer;
pub mod pipeline;
pub mod texture;

#[derive(Default)]
pub struct ResourceTracker {
    next_id: u32, //todo: slotmap instead for better handling of resource clearing upon frame end
    generations: Vec<RenderResourceGeneration>,
}

impl ResourceTracker {
    pub(super) fn clear(&mut self) {
        self.next_id = 0;
        self.generations.clear();
    }

    pub(super) fn new_resource(&mut self) -> RenderResourceId {
        if self.next_id == u32::MAX {
            panic!(
                "No more than {:?} render resources can exist at once across all render graphs",
                u32::MAX
            );
        }
        let id = self.next_id;
        self.next_id += 1;
        self.generations.push(0u16);
        RenderResourceId { id }
    }

    pub(super) fn write(&mut self, id: RenderResourceId) {
        self.generations[id.id as usize] += 1;
    }
}

impl Index<RenderResourceId> for ResourceTracker {
    type Output = RenderResourceGeneration;

    fn index(&self, id: RenderResourceId) -> &Self::Output {
        &self.generations[id.id as usize]
    }
}

pub trait RenderResource: seal::Super {
    type Descriptor: Send + Sync + 'static;
    type Data: Send + Sync + 'static;
    type Store: RenderStore<Self>;

    fn get_store(graph: &RenderGraph, _: seal::Token) -> &Self::Store;
    fn get_store_mut(graph: &mut RenderGraph, _: seal::Token) -> &mut Self::Store;

    fn from_data<'a>(data: &'a Self::Data, world: &'a World) -> Option<&'a Self>;
    fn from_descriptor(
        descriptor: &Self::Descriptor,
        world: &World,
        render_device: &RenderDevice,
    ) -> Self::Data;
}

pub trait RenderStore<R: RenderResource>: seal::Super {
    fn insert(
        &mut self,
        key: RenderResourceId,
        data: RenderResourceInit<R>,
        world: &World,
        render_device: &RenderDevice,
    );

    fn get<'a>(
        &'a self,
        world: &'a World,
        key: RenderResourceId,
    ) -> Option<&'a RenderResourceMeta<R>>;
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

#[derive(Clone)]
pub struct RenderResourceMeta<R: RenderResource> {
    pub(super) descriptor: Option<R::Descriptor>,
    pub(super) resource: R::Data,
}

pub enum RenderResourceInit<R: RenderResource> {
    FromDescriptor(R::Descriptor),
    Resource(RenderResourceMeta<R>),
}

impl<R: RenderResource> From<RenderResourceMeta<R>> for RenderResourceInit<R> {
    fn from(value: RenderResourceMeta<R>) -> Self {
        RenderResourceInit::Resource(value)
    }
}

pub struct SimpleRenderStore<R: RenderResource> {
    resources: HashMap<RenderResourceId, RenderResourceMeta<R>>,
    // resources_to_retain: HashMap<RenderResourceId, InternedRenderLabel>,
    // retained_resources: HashMap<InternedRenderLabel, RenderResourceMeta<R>>,
}

impl<R: RenderResource> seal::Super for SimpleRenderStore<R> {}

impl<R: RenderResource> RenderStore<R> for SimpleRenderStore<R> {
    fn insert(
        &mut self,
        id: RenderResourceId,
        data: RenderResourceInit<R>,
        world: &World,
        render_device: &RenderDevice,
    ) {
        match data {
            RenderResourceInit::FromDescriptor(descriptor) => {
                let resource = R::from_descriptor(&descriptor, world, render_device);
                self.resources.insert(
                    id,
                    RenderResourceMeta {
                        descriptor: Some(descriptor),
                        resource,
                    },
                );
            }
            RenderResourceInit::Resource(meta) => {
                self.resources.insert(id, meta);
            }
        }
    }

    fn get<'a>(
        &'a self,
        _world: &'a World,
        id: RenderResourceId,
    ) -> Option<&'a RenderResourceMeta<R>> {
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

impl<R: RenderResource> Default for SimpleRenderStore<R> {
    fn default() -> Self {
        Self {
            resources: Default::default(),
            //queued_resources: Default::default(),
            // retained_resources: Default::default(),
            // resources_to_retain: Default::default(),
        }
    }
}
pub struct CachedRenderStore<R: RenderResource>
where
    R::Descriptor: Clone + Hash + Eq,
{
    resources: HashMap<RenderResourceId, Arc<RenderResourceMeta<R>>>,
    //queued_resources: HashMap<RenderResourceId, DeferredResourceInit<R>>,
    cached_resources: HashMap<R::Descriptor, Arc<RenderResourceMeta<R>>>,
}

impl<R: RenderResource> seal::Super for CachedRenderStore<R> where R::Descriptor: Clone + Hash + Eq {}

impl<R: RenderResource> RenderStore<R> for CachedRenderStore<R>
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
                self.resources.insert(id, sampler.clone());
            }
            RenderResourceInit::Resource(meta) => {
                if let Some(descriptor) = meta.descriptor.clone() {
                    let meta = Arc::new(meta);
                    self.cached_resources
                        .entry(descriptor)
                        .or_insert(meta.clone());
                    self.resources.insert(id, meta);
                } else {
                    self.resources.insert(id, Arc::new(meta));
                };
            }
        }
    }

    fn get<'a>(
        &'a self,
        world: &'a World,
        id: RenderResourceId,
    ) -> Option<&'a RenderResourceMeta<R>> {
        self.resources.get(&id).map(Borrow::borrow)
    }
}

impl<R: RenderResource> Default for CachedRenderStore<R>
where
    R::Descriptor: Clone + Hash + Eq,
{
    fn default() -> Self {
        Self {
            resources: Default::default(),
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
        RenderResourceInit::Resource(RenderResourceMeta {
            descriptor: None,
            resource: (self)(render_device),
        })
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct RenderResourceId {
    id: u32,
}

pub struct RenderHandle<'a, R> {
    id: RenderResourceId,
    // deps: DependencySet,
    data: PhantomData<&'a R>,
}

impl<'a, R> PartialEq for RenderHandle<'a, R> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id //&& self.deps == other.deps
    }
}

impl<'a, R> Eq for RenderHandle<'a, R> {}

impl<'a, R> Hash for RenderHandle<'a, R> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.data.hash(state);
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
pub struct DependencySet {
    reads: HashSet<RenderResourceId>,
    writes: HashSet<RenderResourceId>,
}

impl DependencySet {
    pub fn add<R: RenderResource>(
        &mut self,
        resource: impl IntoRenderDependency<R>,
    ) -> RenderRef<R> {
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

    pub(super) fn iter_writes<'a>(&'a self) -> impl Iterator<Item = RenderResourceId> + 'a {
        self.writes.iter().copied()
    }
}

struct RenderDependency {
    id: RenderResourceId,
    usage: RenderResourceUsage,
}

#[derive(Copy, Clone)]
enum RenderResourceUsage {
    Read,
    Write,
}

pub trait IntoRenderDependency<R: RenderResource> {
    fn into_render_dependency(self, _: seal::Token) -> impl Iterator<Item = RenderDependency>;
}

impl<R: RenderResource> IntoRenderDependency<R> for &RenderHandle<'_, R> {
    fn into_render_dependency(self, _: seal::Token) -> impl Iterator<Item = RenderDependency> {
        std::iter::once(RenderDependency {
            id: self.id,
            usage: RenderResourceUsage::Read,
        })
    }
}

impl<R: RenderResource> IntoRenderDependency<R> for &mut RenderHandle<'_, R> {
    fn into_render_dependency(self, _: seal::Token) -> impl Iterator<Item = RenderDependency> {
        std::iter::once(RenderDependency {
            id: self.id,
            usage: RenderResourceUsage::Write,
        })
    }
}

pub struct RenderRef<R: RenderResource> {
    id: RenderResourceId,
    data: PhantomData<R>,
}

impl<R: RenderResource> RenderRef<R> {
    pub fn get<'w>(&self, graph: &'w RenderGraph, world: &'w World) -> Option<&'w R> {
        todo!()
    }

    pub fn get_mut<'w>(&self, graph: &'w mut RenderGraph, world: &'w World) -> Option<&'w mut R> {
        todo!()
    }

    pub fn take<'w>(&self, graph: &'w mut RenderGraph, world: &'w World) -> Option<R> {
        todo!()
    }
}

impl<R: RenderResource> Copy for RenderRef<R> {}
impl<R: RenderResource> Clone for RenderRef<R> {
    fn clone(&self) -> Self {
        *self
    }
}
//
// struct MutResource;
// struct RefResource;
// struct TakeResource;
//
// pub trait RenderData<Marker>: 'static {
//     type Item<'w>;
//     type Handle: Send + Sync + 'static;
//
//     fn add_dependencies(handle: &Self::Handle, dependencies: &mut DependencySet);
//     fn get_from_graph<'w>(
//         fetch: &Self::Handle,
//         graph: &'w mut RenderGraph,
//         world: &'w World,
//     ) -> Option<Self::Item<'w>>;
// }
//
// impl<R: RenderResource> RenderData<RefResource> for &'static R {
//     type Item<'w> = &'w R;
//     type Handle = UnsafeRenderHandle<R>;
//
//     fn add_dependencies(handle: &Self::Handle, dependencies: &mut DependencySet) {
//         dependencies.add(handle)
//     }
//
//     fn get_from_graph<'w>(
//         handle: &Self::Handle,
//         graph: &'w mut RenderGraph,
//         world: &'w World,
//     ) -> Option<Self::Item<'w>> {
//         handle.get(graph, world)
//     }
// }
//
// impl<R: RenderResource> RenderData<MutResource> for &'static mut R {
//     type Item<'w> = &'w mut R;
//     type Handle = UnsafeRenderHandle<R>;
//
//     fn add_dependencies(handle: &Self::Handle, dependencies: &mut DependencySet) {
//         dependencies.add(handle)
//     }
//
//     fn get_from_graph<'w>(
//         handle: &Self::Handle,
//         graph: &'w mut RenderGraph,
//         world: &'w World,
//     ) -> Option<Self::Item<'w>> {
//         handle.get_mut(graph, world)
//     }
// }
//
// impl<R: RenderResource> RenderData<TakeResource> for R {
//     type Item<'w> = R;
//     type Handle = UnsafeRenderHandle<R>;
//
//     fn add_dependencies(handle: &Self::Handle, dependencies: &mut DependencySet) {
//         dependencies.add(handle)
//     }
//
//     fn get_from_graph<'w>(
//         handle: &Self::Handle,
//         graph: &'w mut RenderGraph,
//         world: &'w World,
//     ) -> Option<Self::Item<'w>> {
//         handle.take(graph, world)
//     }
// }

// impl RenderData<RefResource> for &'static BindGroup {
//     type Item<'w> = &'w BindGroup;
//     type Handle = UnsafeRenderBindGroup;
//
//     fn add_dependencies(handle: &Self::Handle, dependencies: &mut DependencySet) {
//         dependencies.add_bind_group(handle);
//     }
//
//     fn get_from_graph<'w>(
//         handle: &Self::Handle,
//         graph: &'w mut RenderGraph,
//         world: &'w World,
//     ) -> Option<Self::Item<'w>> {
//         todo!()
//     }
// }
//
// impl<M, D: RenderData<M>, const N: usize> RenderData<[M; N]> for [D; N] {
//     type Item<'w> = [D::Item<'w>; N];
//     type Handle = [D::Handle; N];
//
//     fn add_dependencies(fetch: &Self::Handle, dependencies: &mut DependencySet) {
//         for f in fetch {
//             D::add_dependencies(f, dependencies);
//         }
//     }
//
//     fn get_from_graph<'w>(
//         handle: &Self::Handle,
//         graph: &'w mut RenderGraph,
//         world: &'w World,
//     ) -> Option<Self::Item<'w>> {
//         // let mut items: Self::Item<'w> = [];
//         // for i in 0..N {
//         //     items[i] = D::get_from_graph(&handle[i], graph, world)?;
//         // }
//         // Some(items)
//         todo!()
//     }
// }
//
// macro_rules! impl_render_data {
//     ($(($T: ident, $M:ident, $t: ident, $i: ident)),*) => {
//         impl <$($M,)* $($T: RenderData<$M>),*> RenderData<($($M,)*)> for ($($T,)*) {
//             type Item<'w> = ($($T::Item<'w>,)*);
//             type Handle = ($($T::Handle,)*);
//
//             fn add_dependencies(($($t,)*): &Self::Handle, dependencies: &mut DependencySet) {
//                 $($T::add_dependencies($t, dependencies);)*
//             }
//
//             #[allow(unreachable_patterns)]
//             fn get_from_graph<'w>(
//                 ($($t,)*): &Self::Handle,
//                 graph: &'w mut RenderGraph,
//                 world: &'w World,
//             ) -> Option<Self::Item<'w>> {
//                 $(let $i = $T::get_from_graph($t, graph, world);)*
//                 match ($($i,)*) {
//                     ($(Some($i),)*) => Some(($($i,)*)),
//                     _ => None,
//                 }
//             }
//         }
//     };
// }
//
// all_tuples!(impl_render_data, 0, 16, T, M, t, i);
//
// pub trait IntoRenderData<Marker> {
//     type Data: RenderData<Marker>;
//
//     fn into_render_data(self) -> <Self::Data as RenderData<Marker>>::Handle;
// }
//
// impl<'a, R: RenderResource> IntoRenderData<RefResource> for &RenderHandle<'a, R> {
//     type Data = &'static R;
//
//     fn into_render_data(self) -> <Self::Data as RenderData<RefResource>>::Handle {
//         self.as_unsafe()
//     }
// }
//
// impl<'a, R: RenderResource> IntoRenderData<MutResource> for &mut RenderHandle<'a, R> {
//     type Data = &'static mut R;
//
//     fn into_render_data(self) -> <Self::Data as RenderData<MutResource>>::Handle {
//         self.as_unsafe_mut()
//     }
// }
//
// impl<'a, R: RenderResource> IntoRenderData<TakeResource> for RenderHandle<'a, R> {
//     type Data = R;
//
//     fn into_render_data(self) -> <Self::Data as RenderData<TakeResource>>::Handle {
//         self.into_unsafe()
//     }
// }
//
// impl<'a> IntoRenderData<RefResource> for &RenderBindGroup<'a> {
//     type Data = &'static BindGroup;
//
//     fn into_render_data(self) -> <Self::Data as RenderData<RefResource>>::Handle {
//         self.as_unsafe()
//     }
// }
//
// macro_rules! impl_into_render_data {
//     ($(($T: ident, $M: ident, $t: ident)),*) => {
//         impl <$($M,)* $($T: IntoRenderData<$M>),*> IntoRenderData<($($M,)*)> for ($($T,)*) {
//             type Data = ($($T::Data,)*);
//
//             #[allow(clippy::unused_unit)]
//             fn into_render_data(self) -> <Self::Data as RenderData<($($M,)*)>>::Handle {
//                 let ($($t,)*) = self;
//                 ($($t.into_render_data(),)*)
//             }
//         }
//     }
// }
//
// all_tuples!(impl_into_render_data, 0, 16, T, M, t);

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
