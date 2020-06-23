use crate::resource::{Resource, ResourceSet, ResourceTypeId, Resources};
use crate::schedule::{Runnable, Schedulable};
use bit_set::BitSet;
use derivative::Derivative;
use fxhash::FxHashMap;
use legion_core::{
    borrow::{AtomicRefCell, RefMut},
    command::CommandBuffer,
    cons::{ConsAppend, ConsFlatten},
    filter::EntityFilter,
    index::ArchetypeIndex,
    permission::Permissions,
    query::{Query, Read, View, Write},
    storage::{Component, ComponentTypeId, TagTypeId},
    subworld::{ArchetypeAccess, SubWorld},
    world::{World, WorldId},
};
use std::any::TypeId;
use std::borrow::Cow;
use std::marker::PhantomData;
use tracing::{debug, info, span, Level};

/// Structure describing the resource and component access conditions of the system.
#[derive(Derivative, Debug, Clone)]
#[derivative(Default(bound = ""))]
pub struct SystemAccess {
    pub resources: Permissions<ResourceTypeId>,
    pub components: Permissions<ComponentTypeId>,
    pub tags: Permissions<TagTypeId>,
}

/// This trait is for providing abstraction across tuples of queries for populating the type
/// information in the system closure. This trait also provides access to the underlying query
/// information.
pub trait QuerySet: Send + Sync {
    /// Returns the archetypes accessed by this collection of queries. This allows for caching
    /// effiency and granularity for system dispatching.
    fn filter_archetypes(&mut self, world: &World, archetypes: &mut BitSet);
}

macro_rules! impl_queryset_tuple {
    ($($ty: ident),*) => {
        paste::item! {
            #[allow(unused_parens, non_snake_case)]
            impl<$([<$ty V>], [<$ty F>], )*> QuerySet for ($(Query<[<$ty V>], [<$ty F>]>, )*)
            where
                $([<$ty V>]: for<'v> View<'v>,)*
                $([<$ty F>]: EntityFilter + Send + Sync,)*
            {
                fn filter_archetypes(&mut self, world: &World, bitset: &mut BitSet) {
                    let ($($ty,)*) = self;

                    $(
                        let storage = world.storage();
                        $ty.filter.iter_archetype_indexes(storage).for_each(|ArchetypeIndex(id)| { bitset.insert(id); });
                    )*
                }
            }
        }
    };
}

impl QuerySet for () {
    fn filter_archetypes(&mut self, _: &World, _: &mut BitSet) {}
}

impl<AV, AF> QuerySet for Query<AV, AF>
where
    AV: for<'v> View<'v>,
    AF: EntityFilter + Send + Sync,
{
    fn filter_archetypes(&mut self, world: &World, bitset: &mut BitSet) {
        let storage = world.storage();
        self.filter
            .iter_archetype_indexes(storage)
            .for_each(|ArchetypeIndex(id)| {
                bitset.insert(id);
            });
    }
}

impl_queryset_tuple!(A);
impl_queryset_tuple!(A, B);
impl_queryset_tuple!(A, B, C);
impl_queryset_tuple!(A, B, C, D);
impl_queryset_tuple!(A, B, C, D, E);
impl_queryset_tuple!(A, B, C, D, E, F);
impl_queryset_tuple!(A, B, C, D, E, F, G);
impl_queryset_tuple!(A, B, C, D, E, F, G, H);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SystemId {
    name: Cow<'static, str>,
    type_id: TypeId,
}

struct Unspecified;

impl SystemId {
    pub fn of<T: 'static>(name: Option<String>) -> Self {
        Self {
            name: name
                .unwrap_or_else(|| std::any::type_name::<T>().to_string())
                .into(),
            type_id: TypeId::of::<T>(),
        }
    }

    pub fn name(&self) -> Cow<'static, str> { self.name.clone() }
}

impl std::fmt::Display for SystemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl<T: Into<Cow<'static, str>>> From<T> for SystemId {
    fn from(name: T) -> SystemId {
        SystemId {
            name: name.into(),
            type_id: TypeId::of::<Unspecified>(),
        }
    }
}

/// The concrete type which contains the system closure provided by the user.  This struct should
/// not be instantiated directly, and instead should be created using `SystemBuilder`.
///
/// Implements `Schedulable` which is consumable by the `StageExecutor`, executing the closure.
///
/// Also handles caching of archetype information in a `BitSet`, as well as maintaining the provided
/// information about what queries this system will run and, as a result, its data access.
///
/// Queries are stored generically within this struct, and the `SystemQuery` types are generated
/// on each `run` call, wrapping the world and providing the set to the user in their closure.
pub struct System<R, Q, F>
where
    R: ResourceSet,
    Q: QuerySet,
    F: SystemFn<Resources = <R as ResourceSet>::PreparedResources, Queries = Q>,
{
    pub name: SystemId,
    pub _resources: PhantomData<R>,
    pub queries: AtomicRefCell<Q>,
    pub run_fn: AtomicRefCell<F>,
    pub archetypes: ArchetypeAccess,

    // These are stored statically instead of always iterated and created from the
    // query types, which would make allocations every single request
    pub access: SystemAccess,

    // We pre-allocate a command buffer for ourself. Writes are self-draining so we never have to rellocate.
    pub command_buffer: FxHashMap<WorldId, AtomicRefCell<CommandBuffer>>,
}

impl<R, Q, F> Runnable for System<R, Q, F>
where
    R: ResourceSet,
    Q: QuerySet,
    F: SystemFn<Resources = <R as ResourceSet>::PreparedResources, Queries = Q>,
{
    fn name(&self) -> &SystemId { &self.name }

    fn reads(&self) -> (&[ResourceTypeId], &[ComponentTypeId]) {
        (
            self.access.resources.reads(),
            self.access.components.reads(),
        )
    }
    fn writes(&self) -> (&[ResourceTypeId], &[ComponentTypeId]) {
        (
            self.access.resources.writes(),
            self.access.components.writes(),
        )
    }

    fn prepare(&mut self, world: &World) {
        if let ArchetypeAccess::Some(bitset) = &mut self.archetypes {
            self.queries.get_mut().filter_archetypes(world, bitset);
        }
    }

    fn accesses_archetypes(&self) -> &ArchetypeAccess { &self.archetypes }

    fn command_buffer_mut(&self, world: WorldId) -> Option<RefMut<CommandBuffer>> {
        self.command_buffer.get(&world).map(|cmd| cmd.get_mut())
    }

    unsafe fn run_unsafe(&mut self, world: &World, resources: &Resources) {
        let span = span!(Level::INFO, "System", system = %self.name);
        let _guard = span.enter();

        debug!("Initializing");
        let mut resources = R::fetch_unchecked(resources);
        let mut queries = self.queries.get_mut();
        //let mut prepared_queries = queries.prepare();
        let mut world_shim =
            SubWorld::new_unchecked(world, &self.access.components, &self.archetypes);
        let cmd = self
            .command_buffer
            .entry(world.id())
            .or_insert_with(|| AtomicRefCell::new(CommandBuffer::new(world)));

        info!(permissions = ?self.access, archetypes = ?self.archetypes, "Running");
        use std::ops::DerefMut;
        let mut borrow = self.run_fn.get_mut();
        borrow.deref_mut().run(
            &mut cmd.get_mut(),
            &mut world_shim,
            &mut resources,
            //&mut prepared_queries,
            queries.deref_mut(),
        );
    }
}

/// Supertrait used for defining systems. All wrapper objects for systems implement this trait.
///
/// This trait will generally not be used by users.
pub trait SystemFn {
    type Resources;
    type Queries;

    fn run(
        &mut self,
        commands: &mut CommandBuffer,
        world: &mut SubWorld,
        resources: &mut Self::Resources,
        queries: &mut Self::Queries,
    );
}

pub struct SystemFnWrapper<R, Q, F: FnMut(&mut CommandBuffer, &mut SubWorld, &mut R, &mut Q) + 'static>(
    pub F,
    pub PhantomData<(R, Q)>,
);

impl<F, R, Q> SystemFn for SystemFnWrapper<R, Q, F>
where
    F: FnMut(&mut CommandBuffer, &mut SubWorld, &mut R, &mut Q) + 'static,
{
    type Resources = R;
    type Queries = Q;

    fn run(
        &mut self,
        commands: &mut CommandBuffer,
        world: &mut SubWorld,
        resources: &mut Self::Resources,
        queries: &mut Self::Queries,
    ) {
        (self.0)(commands, world, resources, queries);
    }
}

// This builder uses a Cons/Hlist implemented in cons.rs to generated the static query types
// for this system. Access types are instead stored and abstracted in the top level vec here
// so the underlying ResourceSet type functions from the queries don't need to allocate.
// Otherwise, this leads to excessive alloaction for every call to reads/writes
/// The core builder of `System` types, which are systems within Legion. Systems are implemented
/// as singular closures for a given system - providing queries which should be cached for that
/// system, as well as resource access and other metadata.
/// ```rust
/// # use legion_core::prelude::*;
/// # use legion_systems::prelude::*;
/// # #[derive(Copy, Clone, Debug, PartialEq)]
/// # struct Position;
/// # #[derive(Copy, Clone, Debug, PartialEq)]
/// # struct Velocity;
/// # #[derive(Copy, Clone, Debug, PartialEq)]
/// # struct Model;
/// #[derive(Copy, Clone, Debug, PartialEq)]
/// struct Static;
/// #[derive(Debug)]
/// struct TestResource {}
///
///  let mut system_one = SystemBuilder::<()>::new("TestSystem")
///            .read_resource::<TestResource>()
///            .with_query(<(Read<Position>, Tagged<Model>)>::query()
///                         .filter(!tag::<Static>() | changed::<Position>()))
///            .build(move |commands, world, resource, queries| {
///               let mut count = 0;
///                {
///                    for (entity, pos) in queries.iter_entities_mut(&mut *world) {
///
///                    }
///                }
///            });
/// ```
pub struct SystemBuilder<Q = (), R = ()> {
    name: SystemId,

    queries: Q,
    resources: R,

    resource_access: Permissions<ResourceTypeId>,
    component_access: Permissions<ComponentTypeId>,
    access_all_archetypes: bool,
}

impl SystemBuilder<(), ()> {
    /// Create a new system builder to construct a new system.
    ///
    /// Please note, the `name` argument for this method is just for debugging and visualization
    /// purposes and is not logically used anywhere.
    pub fn new<T: Into<SystemId>>(name: T) -> Self {
        Self {
            name: name.into(),
            queries: (),
            resources: (),
            resource_access: Permissions::default(),
            component_access: Permissions::default(),
            access_all_archetypes: false,
        }
    }
}

impl<Q, R> SystemBuilder<Q, R>
where
    Q: 'static + Send + ConsFlatten,
    R: 'static + Send + ConsFlatten,
{
    /// Defines a query to provide this system for its execution. Multiple queries can be provided,
    /// and queries are cached internally for efficiency for filtering and archetype ID handling.
    ///
    /// It is best practice to define your queries here, to allow for the caching to take place.
    /// These queries are then provided to the executing closure as a tuple of queries.
    pub fn with_query<V, F>(
        mut self,
        query: Query<V, F>,
    ) -> SystemBuilder<<Q as ConsAppend<Query<V, F>>>::Output, R>
    where
        V: for<'a> View<'a>,
        F: 'static + EntityFilter,
        Q: ConsAppend<Query<V, F>>,
    {
        self.component_access.add(V::requires_permissions());

        SystemBuilder {
            name: self.name,
            queries: ConsAppend::append(self.queries, query),
            resources: self.resources,
            resource_access: self.resource_access,
            component_access: self.component_access,
            access_all_archetypes: self.access_all_archetypes,
        }
    }

    /// Flag this resource type as being read by this system.
    ///
    /// This will inform the dispatcher to not allow any writes access to this resource while
    /// this system is running. Parralel reads still occur during execution.
    pub fn read_resource<T>(mut self) -> SystemBuilder<Q, <R as ConsAppend<Read<T>>>::Output>
    where
        T: 'static + Resource,
        R: ConsAppend<Read<T>>,
        <R as ConsAppend<Read<T>>>::Output: ConsFlatten,
    {
        self.resource_access.push_read(ResourceTypeId::of::<T>());

        SystemBuilder {
            name: self.name,
            queries: self.queries,
            resources: ConsAppend::append(self.resources, Read::<T>::default()),
            resource_access: self.resource_access,
            component_access: self.component_access,
            access_all_archetypes: self.access_all_archetypes,
        }
    }

    /// Flag this resource type as being written by this system.
    ///
    /// This will inform the dispatcher to not allow any parallel access to this resource while
    /// this system is running.
    pub fn write_resource<T>(mut self) -> SystemBuilder<Q, <R as ConsAppend<Write<T>>>::Output>
    where
        T: 'static + Resource,
        R: ConsAppend<Write<T>>,
        <R as ConsAppend<Write<T>>>::Output: ConsFlatten,
    {
        self.resource_access.push(ResourceTypeId::of::<T>());

        SystemBuilder {
            name: self.name,
            queries: self.queries,
            resources: ConsAppend::append(self.resources, Write::<T>::default()),
            resource_access: self.resource_access,
            component_access: self.component_access,
            access_all_archetypes: self.access_all_archetypes,
        }
    }

    /// This performs a soft resource block on the component for writing. The dispatcher will
    /// generally handle dispatching read and writes on components based on archetype, allowing
    /// for more granular access and more parallelization of systems.
    ///
    /// Using this method will mark the entire component as read by this system, blocking writing
    /// systems from accessing any archetypes which contain this component for the duration of its
    /// execution.
    ///
    /// This type of access with `SubWorld` is provided for cases where sparse component access
    /// is required and searching entire query spaces for entities is inefficient.
    pub fn read_component<T>(mut self) -> Self
    where
        T: Component,
    {
        self.component_access.push_read(ComponentTypeId::of::<T>());
        self.access_all_archetypes = true;

        self
    }

    /// This performs a exclusive resource block on the component for writing. The dispatcher will
    /// generally handle dispatching read and writes on components based on archetype, allowing
    /// for more granular access and more parallelization of systems.
    ///
    /// Using this method will mark the entire component as written by this system, blocking other
    /// systems from accessing any archetypes which contain this component for the duration of its
    /// execution.
    ///
    /// This type of access with `SubWorld` is provided for cases where sparse component access
    /// is required and searching entire query spaces for entities is inefficient.
    pub fn write_component<T>(mut self) -> Self
    where
        T: Component,
    {
        self.component_access.push(ComponentTypeId::of::<T>());
        self.access_all_archetypes = true;

        self
    }

    /// Builds a standard legion `System`. A system is considered a closure for all purposes. This
    /// closure is `FnMut`, allowing for capture of variables for tracking state for this system.
    /// Instead of the classic OOP architecture of a system, this lets you still maintain state
    /// across execution of the systems while leveraging the type semantics of closures for better
    /// ergonomics.
    pub fn build<F>(self, run_fn: F) -> Box<dyn Schedulable>
    where
        <R as ConsFlatten>::Output: ResourceSet + Send + Sync,
        <Q as ConsFlatten>::Output: QuerySet + Send + Sync,
        <<R as ConsFlatten>::Output as ResourceSet>::PreparedResources: Send + Sync,
        F: FnMut(
                &mut CommandBuffer,
                &mut SubWorld,
                &mut <<R as ConsFlatten>::Output as ResourceSet>::PreparedResources,
                &mut <Q as ConsFlatten>::Output,
            ) + Send
            + Sync
            + 'static,
    {
        let run_fn = SystemFnWrapper(run_fn, PhantomData);
        Box::new(System {
            name: self.name,
            run_fn: AtomicRefCell::new(run_fn),
            _resources: PhantomData::<<R as ConsFlatten>::Output>,
            queries: AtomicRefCell::new(self.queries.flatten()),
            archetypes: if self.access_all_archetypes {
                ArchetypeAccess::All
            } else {
                ArchetypeAccess::Some(BitSet::default())
            },
            access: SystemAccess {
                resources: self.resource_access,
                components: self.component_access,
                tags: Permissions::default(),
            },
            command_buffer: FxHashMap::default(),
        })
    }

    /// Builds a system which is not `Schedulable`, as it is not thread safe (!Send and !Sync),
    /// but still implements all the calling infrastructure of the `Runnable` trait. This provides
    /// a way for legion consumers to leverage the `System` construction and type-handling of
    /// this build for thread local systems which cannot leave the main initializing thread.
    pub fn build_thread_local<F>(self, run_fn: F) -> Box<dyn Runnable>
    where
        <R as ConsFlatten>::Output: ResourceSet + Send + Sync,
        <Q as ConsFlatten>::Output: QuerySet,
        F: FnMut(
                &mut CommandBuffer,
                &mut SubWorld,
                &mut <<R as ConsFlatten>::Output as ResourceSet>::PreparedResources,
                &mut <Q as ConsFlatten>::Output,
            ) + 'static,
    {
        let run_fn = SystemFnWrapper(run_fn, PhantomData);
        Box::new(System {
            name: self.name,
            run_fn: AtomicRefCell::new(run_fn),
            _resources: PhantomData::<<R as ConsFlatten>::Output>,
            queries: AtomicRefCell::new(self.queries.flatten()),
            archetypes: if self.access_all_archetypes {
                ArchetypeAccess::All
            } else {
                ArchetypeAccess::Some(BitSet::default())
            },
            access: SystemAccess {
                resources: self.resource_access,
                components: self.component_access,
                tags: Permissions::default(),
            },
            command_buffer: FxHashMap::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::*;
    use legion_core::prelude::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Pos(f32, f32, f32);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Vel(f32, f32, f32);

    #[derive(Default)]
    struct TestResource(pub i32);
    #[derive(Default)]
    struct TestResourceTwo(pub i32);
    #[derive(Default)]
    struct TestResourceThree(pub i32);
    #[derive(Default)]
    struct TestResourceFour(pub i32);

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct TestComp(f32, f32, f32);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct TestCompTwo(f32, f32, f32);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct TestCompThree(f32, f32, f32);

    #[test]
    fn builder_schedule_execute() {
        let _ = tracing_subscriber::fmt::try_init();

        let universe = Universe::new();
        let mut world = universe.create_world();

        let mut resources = Resources::default();
        resources.insert(TestResource(123));
        resources.insert(TestResourceTwo(123));

        let components = vec![
            (Pos(1., 2., 3.), Vel(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Vel(0.4, 0.5, 0.6)),
        ];

        let mut expected = HashMap::<Entity, (Pos, Vel)>::new();

        for (i, e) in world.insert((), components.clone()).iter().enumerate() {
            if let Some((pos, rot)) = components.get(i) {
                expected.insert(*e, (*pos, *rot));
            }
        }

        #[derive(Debug, Eq, PartialEq)]
        pub enum TestSystems {
            TestSystemOne,
            TestSystemTwo,
            TestSystemThree,
            TestSystemFour,
        }

        let runs = Arc::new(Mutex::new(Vec::new()));

        let system_one_runs = runs.clone();
        let system_one = SystemBuilder::<()>::new("TestSystem1")
            .read_resource::<TestResource>()
            .with_query(Read::<Pos>::query())
            .with_query(Write::<Vel>::query())
            .build(move |_commands, _world, _resource, _queries| {
                tracing::trace!("system_one");
                system_one_runs
                    .lock()
                    .unwrap()
                    .push(TestSystems::TestSystemOne);
            });

        let system_two_runs = runs.clone();
        let system_two = SystemBuilder::<()>::new("TestSystem2")
            .write_resource::<TestResourceTwo>()
            .with_query(Read::<Vel>::query())
            .build(move |_commands, _world, _resource, _queries| {
                tracing::trace!("system_two");
                system_two_runs
                    .lock()
                    .unwrap()
                    .push(TestSystems::TestSystemTwo);
            });

        let system_three_runs = runs.clone();
        let system_three = SystemBuilder::<()>::new("TestSystem3")
            .read_resource::<TestResourceTwo>()
            .with_query(Read::<Vel>::query())
            .build(move |_commands, _world, _resource, _queries| {
                tracing::trace!("system_three");
                system_three_runs
                    .lock()
                    .unwrap()
                    .push(TestSystems::TestSystemThree);
            });
        let system_four_runs = runs.clone();
        let system_four = SystemBuilder::<()>::new("TestSystem4")
            .write_resource::<TestResourceTwo>()
            .with_query(Read::<Vel>::query())
            .build(move |_commands, _world, _resource, _queries| {
                tracing::trace!("system_four");
                system_four_runs
                    .lock()
                    .unwrap()
                    .push(TestSystems::TestSystemFour);
            });

        let order = vec![
            TestSystems::TestSystemOne,
            TestSystems::TestSystemTwo,
            TestSystems::TestSystemThree,
            TestSystems::TestSystemFour,
        ];

        let systems = vec![system_one, system_two, system_three, system_four];

        let mut executor = Executor::new(systems);
        executor.execute(&mut world, &mut resources);

        assert_eq!(*(runs.lock().unwrap()), order);
    }

    #[test]
    fn builder_create_and_execute() {
        let _ = tracing_subscriber::fmt::try_init();

        let universe = Universe::new();
        let mut world = universe.create_world();

        let mut resources = Resources::default();
        resources.insert(TestResource(123));

        let components = vec![
            (Pos(1., 2., 3.), Vel(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Vel(0.4, 0.5, 0.6)),
        ];

        let mut expected = HashMap::<Entity, (Pos, Vel)>::new();

        for (i, e) in world.insert((), components.clone()).iter().enumerate() {
            if let Some((pos, rot)) = components.get(i) {
                expected.insert(*e, (*pos, *rot));
            }
        }

        let mut system = SystemBuilder::<()>::new("TestSystem")
            .read_resource::<TestResource>()
            .with_query(Read::<Pos>::query())
            .with_query(Read::<Vel>::query())
            .build(move |_commands, world, resource, queries| {
                assert_eq!(resource.0, 123);
                let mut count = 0;
                {
                    for (entity, pos) in queries.0.iter_entities(world) {
                        assert_eq!(expected.get(&entity).unwrap().0, *pos);
                        count += 1;
                    }
                }

                assert_eq!(components.len(), count);
            });
        system.prepare(&world);
        system.run(&mut world, &mut resources);
    }

    #[test]
    fn fnmut_stateful_system_test() {
        let _ = tracing_subscriber::fmt::try_init();

        let universe = Universe::new();
        let mut world = universe.create_world();

        let mut resources = Resources::default();
        resources.insert(TestResource(123));

        let components = vec![
            (Pos(1., 2., 3.), Vel(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Vel(0.4, 0.5, 0.6)),
        ];

        let mut expected = HashMap::<Entity, (Pos, Vel)>::new();

        for (i, e) in world.insert((), components.clone()).iter().enumerate() {
            if let Some((pos, rot)) = components.get(i) {
                expected.insert(*e, (*pos, *rot));
            }
        }

        let mut system = SystemBuilder::<()>::new("TestSystem")
            .read_resource::<TestResource>()
            .with_query(Read::<Pos>::query())
            .with_query(Read::<Vel>::query())
            .build(move |_, _, _, _| {});

        system.prepare(&world);
        system.run(&mut world, &mut resources);
    }

    #[test]
    fn system_mutate_archetype() {
        let _ = tracing_subscriber::fmt::try_init();

        let universe = Universe::new();
        let mut world = universe.create_world();
        let mut resources = Resources::default();

        #[derive(Default, Clone, Copy)]
        pub struct Balls(u32);

        let components = vec![
            (Pos(1., 2., 3.), Vel(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Vel(0.4, 0.5, 0.6)),
        ];

        let mut expected = HashMap::<Entity, (Pos, Vel)>::new();

        for (i, e) in world.insert((), components.clone()).iter().enumerate() {
            if let Some((pos, rot)) = components.get(i) {
                expected.insert(*e, (*pos, *rot));
            }
        }

        let expected_copy = expected.clone();
        let mut system = SystemBuilder::<()>::new("TestSystem")
            .with_query(<(Read<Pos>, Read<Vel>)>::query())
            .build(move |_, world, _, query| {
                let mut count = 0;
                {
                    for (entity, (pos, vel)) in query.iter_entities(world) {
                        assert_eq!(expected_copy.get(&entity).unwrap().0, *pos);
                        assert_eq!(expected_copy.get(&entity).unwrap().1, *vel);
                        count += 1;
                    }
                }

                assert_eq!(components.len(), count);
            });

        system.prepare(&world);
        system.run(&mut world, &mut resources);

        world
            .add_component(*(expected.keys().nth(0).unwrap()), Balls::default())
            .unwrap();

        system.prepare(&world);
        system.run(&mut world, &mut resources);
    }

    #[test]
    fn system_mutate_archetype_buffer() {
        let _ = tracing_subscriber::fmt::try_init();

        let universe = Universe::new();
        let mut world = universe.create_world();
        let mut resources = Resources::default();

        #[derive(Default, Clone, Copy)]
        pub struct Balls(u32);

        let components = (0..30000)
            .map(|_| (Pos(1., 2., 3.), Vel(0.1, 0.2, 0.3)))
            .collect::<Vec<_>>();

        let mut expected = HashMap::<Entity, (Pos, Vel)>::new();

        for (i, e) in world.insert((), components.clone()).iter().enumerate() {
            if let Some((pos, rot)) = components.get(i) {
                expected.insert(*e, (*pos, *rot));
            }
        }

        let expected_copy = expected.clone();
        let mut system = SystemBuilder::<()>::new("TestSystem")
            .with_query(<(Read<Pos>, Read<Vel>)>::query())
            .build(move |command_buffer, world, _, query| {
                let mut count = 0;
                {
                    for (entity, (pos, vel)) in query.iter_entities(world) {
                        assert_eq!(expected_copy.get(&entity).unwrap().0, *pos);
                        assert_eq!(expected_copy.get(&entity).unwrap().1, *vel);
                        count += 1;

                        command_buffer.add_component(entity, Balls::default());
                    }
                }

                assert_eq!(components.len(), count);
            });

        system.prepare(&world);
        system.run(&mut world, &mut resources);

        system
            .command_buffer_mut(world.id())
            .unwrap()
            .write(&mut world);

        system.prepare(&world);
        system.run(&mut world, &mut resources);
    }

    #[test]
    #[cfg(feature = "par-schedule")]
    fn par_res_write() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let _ = tracing_subscriber::fmt::try_init();

        #[derive(Default)]
        struct AtomicRes(AtomicRefCell<AtomicUsize>);

        let universe = Universe::new();
        let mut world = universe.create_world();

        let mut resources = Resources::default();
        resources.insert(AtomicRes::default());

        let system1 = SystemBuilder::<()>::new("TestSystem1")
            .write_resource::<AtomicRes>()
            .with_query(Read::<Pos>::query())
            .with_query(Read::<Vel>::query())
            .build(move |_, _, resource, _| {
                resource.0.get_mut().fetch_add(1, Ordering::SeqCst);
            });

        let system2 = SystemBuilder::<()>::new("TestSystem2")
            .write_resource::<AtomicRes>()
            .with_query(Read::<Pos>::query())
            .with_query(Read::<Vel>::query())
            .build(move |_, _, resource, _| {
                resource.0.get_mut().fetch_add(1, Ordering::SeqCst);
            });

        let system3 = SystemBuilder::<()>::new("TestSystem3")
            .write_resource::<AtomicRes>()
            .with_query(Read::<Pos>::query())
            .with_query(Read::<Vel>::query())
            .build(move |_, _, resource, _| {
                resource.0.get_mut().fetch_add(1, Ordering::SeqCst);
            });

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(8)
            .build()
            .unwrap();

        tracing::debug!(
            reads = ?system1.reads(),
            writes = ?system1.writes(),
            "System access"
        );

        let systems = vec![system1, system2, system3];
        let mut executor = Executor::new(systems);
        pool.install(|| {
            for _ in 0..1000 {
                executor.execute(&mut world, &mut resources);
            }
        });

        assert_eq!(
            resources
                .get::<AtomicRes>()
                .unwrap()
                .0
                .get()
                .load(Ordering::SeqCst),
            3 * 1000,
        );
    }

    #[test]
    #[cfg(feature = "par-schedule")]
    fn par_res_readwrite() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let _ = tracing_subscriber::fmt::try_init();

        #[derive(Default)]
        struct AtomicRes(AtomicRefCell<AtomicUsize>);

        let universe = Universe::new();
        let mut world = universe.create_world();

        let mut resources = Resources::default();
        resources.insert(AtomicRes::default());

        let system1 = SystemBuilder::<()>::new("TestSystem1")
            .read_resource::<AtomicRes>()
            .with_query(Read::<Pos>::query())
            .with_query(Read::<Vel>::query())
            .build(move |_, _, resource, _| {
                resource.0.get().fetch_add(1, Ordering::SeqCst);
            });

        let system2 = SystemBuilder::<()>::new("TestSystem2")
            .write_resource::<AtomicRes>()
            .with_query(Read::<Pos>::query())
            .with_query(Read::<Vel>::query())
            .build(move |_, _, resource, _| {
                resource.0.get_mut().fetch_add(1, Ordering::SeqCst);
            });

        let system3 = SystemBuilder::<()>::new("TestSystem3")
            .write_resource::<AtomicRes>()
            .with_query(Read::<Pos>::query())
            .with_query(Read::<Vel>::query())
            .build(move |_, _, resource, _| {
                resource.0.get_mut().fetch_add(1, Ordering::SeqCst);
            });

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(8)
            .build()
            .unwrap();

        tracing::debug!(
            reads = ?system1.reads(),
            writes = ?system1.writes(),
            "System access"
        );

        let systems = vec![system1, system2, system3];
        let mut executor = Executor::new(systems);
        pool.install(|| {
            for _ in 0..1000 {
                executor.execute(&mut world, &mut resources);
            }
        });
    }

    #[test]
    #[cfg(feature = "par-schedule")]
    #[allow(clippy::float_cmp)]
    fn par_comp_readwrite() {
        let _ = tracing_subscriber::fmt::try_init();

        let universe = Universe::new();
        let mut world = universe.create_world();

        #[derive(Clone, Copy, Debug, PartialEq)]
        struct Comp1(f32, f32, f32);
        #[derive(Clone, Copy, Debug, PartialEq)]
        struct Comp2(f32, f32, f32);

        let components = vec![
            (Comp1(69., 69., 69.), Comp2(69., 69., 69.)),
            (Comp1(69., 69., 69.), Comp2(69., 69., 69.)),
        ];

        let mut expected = HashMap::<Entity, (Comp1, Comp2)>::new();

        for (i, e) in world.insert((), components.clone()).iter().enumerate() {
            if let Some((pos, rot)) = components.get(i) {
                expected.insert(*e, (*pos, *rot));
            }
        }

        let system1 = SystemBuilder::<()>::new("TestSystem1")
            .with_query(<(Read<Comp1>, Read<Comp2>)>::query())
            .build(move |_, world, _, query| {
                query.iter(world).for_each(|(one, two)| {
                    assert_eq!(one.0, 69.);
                    assert_eq!(one.1, 69.);
                    assert_eq!(one.2, 69.);

                    assert_eq!(two.0, 69.);
                    assert_eq!(two.1, 69.);
                    assert_eq!(two.2, 69.);
                });
            });

        let system2 = SystemBuilder::<()>::new("TestSystem2")
            .with_query(<(Write<Comp1>, Read<Comp2>)>::query())
            .build(move |_, world, _, query| {
                query.iter_mut(world).for_each(|(mut one, two)| {
                    one.0 = 456.;
                    one.1 = 456.;
                    one.2 = 456.;

                    assert_eq!(two.0, 69.);
                    assert_eq!(two.1, 69.);
                    assert_eq!(two.2, 69.);
                });
            });

        let system3 = SystemBuilder::<()>::new("TestSystem3")
            .with_query(<(Write<Comp1>, Write<Comp2>)>::query())
            .build(move |_, world, _, query| {
                query.iter_mut(world).for_each(|(mut one, mut two)| {
                    assert_eq!(one.0, 456.);
                    assert_eq!(one.1, 456.);
                    assert_eq!(one.2, 456.);

                    assert_eq!(two.0, 69.);
                    assert_eq!(two.1, 69.);
                    assert_eq!(two.2, 69.);

                    one.0 = 789.;
                    one.1 = 789.;
                    one.2 = 789.;

                    two.0 = 789.;
                    two.1 = 789.;
                    two.2 = 789.;
                });
            });

        let system4 = SystemBuilder::<()>::new("TestSystem4")
            .with_query(<(Read<Comp1>, Read<Comp2>)>::query())
            .build(move |_, world, _, query| {
                query.iter(world).for_each(|(one, two)| {
                    assert_eq!(one.0, 789.);
                    assert_eq!(one.1, 789.);
                    assert_eq!(one.2, 789.);

                    assert_eq!(two.0, 789.);
                    assert_eq!(two.1, 789.);
                    assert_eq!(two.2, 789.);
                });
            });

        let system5 = SystemBuilder::<()>::new("TestSystem5")
            .with_query(<(Write<Comp1>, Write<Comp2>)>::query())
            .build(move |_, world, _, query| {
                query.iter_mut(world).for_each(|(mut one, mut two)| {
                    assert_eq!(one.0, 789.);
                    assert_eq!(one.1, 789.);
                    assert_eq!(one.2, 789.);

                    assert_eq!(two.0, 789.);
                    assert_eq!(two.1, 789.);
                    assert_eq!(two.2, 789.);

                    one.0 = 69.;
                    one.1 = 69.;
                    one.2 = 69.;

                    two.0 = 69.;
                    two.1 = 69.;
                    two.2 = 69.;
                });
            });

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(8)
            .build()
            .unwrap();

        tracing::debug!(
            reads = ?system1.reads(),
            writes = ?system1.writes(),
            "System access"
        );

        let systems = vec![system1, system2, system3, system4, system5];
        let mut executor = Executor::new(systems);
        pool.install(|| {
            for _ in 0..1000 {
                executor.execute(&mut world, &mut Resources::default());
            }
        });
    }

    #[test]
    fn split_world() {
        let mut world = World::new();

        let system = SystemBuilder::new("split worlds")
            .with_query(Write::<usize>::query())
            .with_query(Write::<bool>::query())
            .build(|_, world, _, (query_a, query_b)| {
                let (mut left, mut right) = world.split_for_query(&query_a);
                for _ in query_a.iter_mut(&mut left) {
                    let _ = query_b.iter_mut(&mut right);
                }
            });

        let mut schedule = Schedule::builder().add_system(system).build();
        schedule.execute(&mut world, &mut Resources::default());
    }

    #[test]
    fn split_world2() {
        let system = SystemBuilder::new("system")
            .with_query(<(Read<usize>, Write<isize>)>::query())
            .write_component::<bool>()
            .build_thread_local(move |_, world, _, query| {
                let (_, mut world) = world.split_for_query(&query);
                let (_, _) = world.split::<Write<bool>>();
            });

        let mut schedule = Schedule::builder().add_thread_local(system).build();

        let mut world = World::new();
        schedule.execute(&mut world, &mut Resources::default());
    }

    #[test]
    fn overlapped_reads() {
        let _ = tracing_subscriber::fmt::try_init();

        #[derive(Debug)]
        struct Money(f64);
        #[derive(Debug)]
        struct Health(f64);
        struct Food(f64);

        let universe = Universe::new();
        let mut world = universe.create_world();

        world.insert((), vec![(Money(5.0), Food(5.0))]);

        world.insert(
            (),
            vec![
                (Money(4.0), Health(3.0)),
                (Money(4.0), Health(3.0)),
                (Money(4.0), Health(3.0)),
            ],
        );

        let show_me_the_money = SystemBuilder::new("money_show")
            .with_query(<(Read<Money>, Read<Food>)>::query())
            .build(|_, world, _, query| {
                for (money, _food) in query.iter(world) {
                    info!("Look at my money {:?}", money);
                }
            });

        let health_conscious = SystemBuilder::new("healthy")
            .with_query(<(Read<Money>, Read<Health>)>::query())
            .build(|_, world, _, query| {
                for (_money, health) in query.iter(world) {
                    info!("So healthy {:?}", health);
                }
            });

        let mut schedule = Schedule::builder()
            .add_system(show_me_the_money)
            .flush()
            .add_system(health_conscious)
            .flush()
            .build();

        let mut resources = Resources::default();
        schedule.execute(&mut world, &mut resources);
    }

    #[test]
    fn thread_local_query() {
        let mut world = World::default();
        let _ = world.insert((), Some((1f32,)));

        let system = SystemBuilder::new("system")
            .with_query(<Write<f32>>::query())
            .build_thread_local(move |_, world, _, query| for _ in query.iter_mut(world) {});

        let mut schedule = Schedule::builder().add_thread_local(system).build();
        schedule.execute(&mut world, &mut Resources::default());
    }
}
