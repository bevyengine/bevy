pub use super::Query;
use crate::{
    resource::{FetchResource, ResourceQuery, Resources, UnsafeClone},
    system::{Commands, System, SystemId, ThreadLocalExecution},
    QueryAccess, QuerySet, QueryTuple, TypeAccess,
};
use bevy_hecs::{ArchetypeComponent, Fetch, Query as HecsQuery, World};
use std::{any::TypeId, borrow::Cow};

#[derive(Debug)]
pub(crate) struct SystemFn<State, F, ThreadLocalF, Init, Update>
where
    F: FnMut(&World, &Resources, &mut State) + Send + Sync,
    ThreadLocalF: FnMut(&mut World, &mut Resources, &mut State) + Send + Sync,
    Init: FnMut(&mut World, &mut Resources, &mut State) + Send + Sync,
    Update: FnMut(&World, &mut TypeAccess<ArchetypeComponent>, &mut State) + Send + Sync,
    State: Send + Sync,
{
    pub state: State,
    pub func: F,
    pub thread_local_func: ThreadLocalF,
    pub init_func: Init,
    pub thread_local_execution: ThreadLocalExecution,
    pub resource_access: TypeAccess<TypeId>,
    pub name: Cow<'static, str>,
    pub id: SystemId,
    pub archetype_component_access: TypeAccess<ArchetypeComponent>,
    pub update_func: Update,
}

impl<State, F, ThreadLocalF, Init, Update> System for SystemFn<State, F, ThreadLocalF, Init, Update>
where
    F: FnMut(&World, &Resources, &mut State) + Send + Sync,
    ThreadLocalF: FnMut(&mut World, &mut Resources, &mut State) + Send + Sync,
    Init: FnMut(&mut World, &mut Resources, &mut State) + Send + Sync,
    Update: FnMut(&World, &mut TypeAccess<ArchetypeComponent>, &mut State) + Send + Sync,
    State: Send + Sync,
{
    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn update(&mut self, world: &World) {
        (self.update_func)(world, &mut self.archetype_component_access, &mut self.state);
    }

    fn archetype_component_access(&self) -> &TypeAccess<ArchetypeComponent> {
        &self.archetype_component_access
    }

    fn resource_access(&self) -> &TypeAccess<TypeId> {
        &self.resource_access
    }

    fn thread_local_execution(&self) -> ThreadLocalExecution {
        self.thread_local_execution
    }

    #[inline]
    fn run(&mut self, world: &World, resources: &Resources) {
        (self.func)(world, resources, &mut self.state);
    }

    fn run_thread_local(&mut self, world: &mut World, resources: &mut Resources) {
        (self.thread_local_func)(world, resources, &mut self.state);
    }

    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        (self.init_func)(world, resources, &mut self.state);
    }

    fn id(&self) -> SystemId {
        self.id
    }
}

/// Converts `Self` into a For-Each system
pub trait IntoForEachSystem<CommandBuffer, R, C> {
    fn system(self) -> Box<dyn System>;
}

struct ForEachState {
    commands: Commands,
    query_access: QueryAccess,
}

macro_rules! impl_into_foreach_system {
    (($($commands: ident)*), ($($resource: ident),*), ($($component: ident),*)) => {
        impl<Func, $($resource,)* $($component,)*> IntoForEachSystem<($($commands,)*), ($($resource,)*), ($($component,)*)> for Func
        where
            Func:
                FnMut($($commands,)* $($resource,)* $($component,)*) +
                FnMut(
                    $($commands,)*
                    $(<<$resource as ResourceQuery>::Fetch as FetchResource>::Item,)*
                    $(<<$component as HecsQuery>::Fetch as Fetch>::Item,)*)+
                Send + Sync + 'static,
            $($component: HecsQuery,)*
            $($resource: ResourceQuery,)*
        {
            #[allow(non_snake_case)]
            #[allow(unused_variables)]
            #[allow(unused_unsafe)]
            fn system(mut self) -> Box<dyn System> {
                let id = SystemId::new();
                Box::new(SystemFn {
                    state: ForEachState {
                        commands: Commands::default(),
                        query_access: <($($component,)*) as HecsQuery>::Fetch::access(),
                    },
                    thread_local_execution: ThreadLocalExecution::NextFlush,
                    name: core::any::type_name::<Self>().into(),
                    id,
                    func: move |world, resources, state| {
                        {
                            let state_commands = &state.commands;
                            if let Some(($($resource,)*)) = resources.query_system::<($($resource,)*)>(id) {
                                // SAFE: the scheduler has ensured that there is no archetype clashing here
                                unsafe {
                                    for ($($component,)*) in world.query_unchecked::<($($component,)*)>() {
                                        fn_call!(self, ($($commands, state_commands)*), ($($resource),*), ($($component),*))
                                    }
                                }
                            }
                        }
                    },
                    thread_local_func: move |world, resources, state| {
                        state.commands.apply(world, resources);
                    },
                    init_func: move |world, resources, state| {
                        <($($resource,)*)>::initialize(resources, Some(id));
                        state.commands.set_entity_reserver(world.get_entity_reserver())
                    },
                    resource_access: <<($($resource,)*) as ResourceQuery>::Fetch as FetchResource>::access(),
                    archetype_component_access: TypeAccess::default(),
                    update_func: |world, archetype_component_access, state| {
                        archetype_component_access.clear();
                        state.query_access.get_world_archetype_access(world, Some(archetype_component_access));
                    },
                })
            }
        }
    };
}

struct QuerySystemState {
    query_accesses: Vec<Vec<QueryAccess>>,
    query_type_names: Vec<&'static str>,
    archetype_component_accesses: Vec<TypeAccess<ArchetypeComponent>>,
    commands: Commands,
}

/// Converts `Self` into a Query System
pub trait IntoQuerySystem<Commands, R, Q, QS> {
    fn system(self) -> Box<dyn System>;
}

macro_rules! impl_into_query_system {
    (($($commands: ident)*), ($($resource: ident),*), ($($query: ident),*), ($($query_set: ident),*)) => {
        impl<Func, $($resource,)* $($query,)* $($query_set,)*> IntoQuerySystem<($($commands,)*), ($($resource,)*), ($($query,)*), ($($query_set,)*)> for Func where
            Func:
                FnMut($($commands,)* $($resource,)* $(Query<$query>,)* $(QuerySet<$query_set>,)*) +
                FnMut(
                    $($commands,)*
                    $(<<$resource as ResourceQuery>::Fetch as FetchResource>::Item,)*
                    $(Query<$query>,)*
                    $(QuerySet<$query_set>,)*
                ) +
                Send + Sync +'static,
            $($query: HecsQuery,)*
            $($query_set: QueryTuple,)*
            $($resource: ResourceQuery,)*
        {
            #[allow(non_snake_case)]
            #[allow(unused_variables)]
            #[allow(unused_unsafe)]
            #[allow(unused_assignments)]
            #[allow(unused_mut)]
            fn system(mut self) -> Box<dyn System> {
                let id = SystemId::new();
                let query_accesses = vec![
                    $(vec![<$query::Fetch as Fetch>::access()],)*
                    $($query_set::get_accesses(),)*
                ];
                let query_type_names = vec![
                    $(std::any::type_name::<$query>(),)*
                    $(std::any::type_name::<$query_set>(),)*
                ];
                let archetype_component_accesses = vec![TypeAccess::default(); query_accesses.len()];
                Box::new(SystemFn {
                    state: QuerySystemState {
                        query_accesses,
                        query_type_names,
                        archetype_component_accesses,
                        commands: Commands::default(),
                    },
                    thread_local_execution: ThreadLocalExecution::NextFlush,
                    id,
                    name: core::any::type_name::<Self>().into(),
                    func: move |world, resources, state| {
                        {
                            if let Some(($($resource,)*)) = resources.query_system::<($($resource,)*)>(id) {
                                let mut i = 0;
                                $(
                                    let $query = Query::<$query>::new(
                                        world,
                                        &state.archetype_component_accesses[i]
                                    );
                                    i += 1;
                                )*
                                $(
                                    let $query_set = QuerySet::<$query_set>::new(
                                        world,
                                        &state.archetype_component_accesses[i]
                                    );
                                    i += 1;
                                )*

                                let commands = &state.commands;
                                fn_call!(self, ($($commands, commands)*), ($($resource),*), ($($query),*), ($($query_set),*))
                            }
                        }
                    },
                    thread_local_func: move |world, resources, state| {
                        state.commands.apply(world, resources);
                    },
                    init_func: move |world, resources, state| {
                        <($($resource,)*)>::initialize(resources, Some(id));
                        state.commands.set_entity_reserver(world.get_entity_reserver())

                    },
                    resource_access: <<($($resource,)*) as ResourceQuery>::Fetch as FetchResource>::access(),
                    archetype_component_access: TypeAccess::default(),
                    update_func: |world, archetype_component_access, state| {
                        archetype_component_access.clear();
                        let mut conflict_index = None;
                        let mut conflict_name = None;
                        for (i, (query_accesses, component_access)) in state.query_accesses.iter().zip(state.archetype_component_accesses.iter_mut()).enumerate() {
                            component_access.clear();
                            for query_access in query_accesses.iter() {
                                query_access.get_world_archetype_access(world, Some(component_access));
                            }
                            if !component_access.is_compatible(archetype_component_access) {
                                conflict_index = Some(i);
                                conflict_name = component_access.get_conflict(archetype_component_access).and_then(|archetype_component|
                                    query_accesses
                                        .iter()
                                        .filter_map(|query_access| query_access.get_type_name(archetype_component.component))
                                        .next());
                                break;
                            }
                            archetype_component_access.union(component_access);
                        }
                        if let Some(conflict_index) = conflict_index {
                            let mut conflicts_with_index = None;
                            for prior_index in 0..conflict_index {
                                if !state.archetype_component_accesses[conflict_index].is_compatible(&state.archetype_component_accesses[prior_index]) {
                                    conflicts_with_index = Some(prior_index);
                                }
                            }
                            panic!("System {} has conflicting queries. {} conflicts with the component access [{}] in this prior query: {}",
                                core::any::type_name::<Self>(),
                                state.query_type_names[conflict_index],
                                conflict_name.unwrap_or("Unknown"),
                                conflicts_with_index.map(|index| state.query_type_names[index]).unwrap_or("Unknown"));
                        }
                    },
                })
            }
        }
    };
}

macro_rules! fn_call {
    ($self:ident, ($($commands: ident, $commands_var: ident)*), ($($resource: ident),*), ($($a: ident),*), ($($b: ident),*)) => {
        unsafe { $self($($commands_var.clone(),)* $($resource.unsafe_clone(),)* $($a,)* $($b,)*) }
    };
    ($self:ident, ($($commands: ident, $commands_var: ident)*), ($($resource: ident),*), ($($a: ident),*)) => {
        unsafe { $self($($commands_var.clone(),)* $($resource.unsafe_clone(),)* $($a,)*) }
    };
    ($self:ident, (), ($($resource: ident),*), ($($a: ident),*)) => {
        unsafe { $self($($resource.unsafe_clone(),)* $($a,)*) }
    };
}

macro_rules! impl_into_query_systems {
    (($($resource: ident,)*), ($($query: ident),*)) => {
        #[rustfmt::skip]
        impl_into_query_system!((), ($($resource),*), ($($query),*), ());
        #[rustfmt::skip]
        impl_into_query_system!((), ($($resource),*), ($($query),*), (QS1));
        #[rustfmt::skip]
        impl_into_query_system!((), ($($resource),*), ($($query),*), (QS1, QS2));

        #[rustfmt::skip]
        impl_into_query_system!((Commands), ($($resource),*), ($($query),*), ());
        #[rustfmt::skip]
        impl_into_query_system!((Commands), ($($resource),*), ($($query),*), (QS1));
        #[rustfmt::skip]
        impl_into_query_system!((Commands), ($($resource),*), ($($query),*), (QS1, QS2));
    }
}

macro_rules! impl_into_foreach_systems {
    (($($resource: ident,)*), ($($component: ident),*)) => {
        #[rustfmt::skip]
        impl_into_foreach_system!((), ($($resource),*), ($($component),*));
        #[rustfmt::skip]
        impl_into_foreach_system!((Commands), ($($resource),*), ($($component),*));
    }
}

macro_rules! impl_into_systems {
    ($($resource: ident),*) => {
        #[rustfmt::skip]
        impl_into_foreach_systems!(($($resource,)*), (A));
        #[rustfmt::skip]
        impl_into_foreach_systems!(($($resource,)*), (A,B));
        #[rustfmt::skip]
        impl_into_foreach_systems!(($($resource,)*), (A,B,C));
        #[rustfmt::skip]
        impl_into_foreach_systems!(($($resource,)*), (A,B,C,D));
        #[rustfmt::skip]
        impl_into_foreach_systems!(($($resource,)*), (A,B,C,D,E));
        #[rustfmt::skip]
        impl_into_foreach_systems!(($($resource,)*), (A,B,C,D,E,F));
        #[rustfmt::skip]
        impl_into_foreach_systems!(($($resource,)*), (A,B,C,D,E,F,G));
        #[rustfmt::skip]
        impl_into_foreach_systems!(($($resource,)*), (A,B,C,D,E,F,G,H));

        #[rustfmt::skip]
        impl_into_query_systems!(($($resource,)*), ());
        #[rustfmt::skip]
        impl_into_query_systems!(($($resource,)*), (A));
        #[rustfmt::skip]
        impl_into_query_systems!(($($resource,)*), (A,B));
        #[rustfmt::skip]
        impl_into_query_systems!(($($resource,)*), (A,B,C));
        #[rustfmt::skip]
        impl_into_query_systems!(($($resource,)*), (A,B,C,D));
        #[rustfmt::skip]
        impl_into_query_systems!(($($resource,)*), (A,B,C,D,E));
        #[rustfmt::skip]
        impl_into_query_systems!(($($resource,)*), (A,B,C,D,E,F));
    };
}

#[rustfmt::skip]
impl_into_systems!();
#[rustfmt::skip]
impl_into_systems!(Ra);
#[rustfmt::skip]
impl_into_systems!(Ra,Rb);
#[rustfmt::skip]
impl_into_systems!(Ra,Rb,Rc);
#[rustfmt::skip]
impl_into_systems!(Ra,Rb,Rc,Rd);
#[rustfmt::skip]
impl_into_systems!(Ra,Rb,Rc,Rd,Re);
#[rustfmt::skip]
impl_into_systems!(Ra,Rb,Rc,Rd,Re,Rf);
#[rustfmt::skip]
impl_into_systems!(Ra,Rb,Rc,Rd,Re,Rf,Rg);
#[rustfmt::skip]
impl_into_systems!(Ra,Rb,Rc,Rd,Re,Rf,Rg,Rh);
#[rustfmt::skip]
impl_into_systems!(Ra,Rb,Rc,Rd,Re,Rf,Rg,Rh,Ri);
#[rustfmt::skip]
impl_into_systems!(Ra,Rb,Rc,Rd,Re,Rf,Rg,Rh,Ri,Rj);

/// Converts `Self` into a thread local system
pub trait IntoThreadLocalSystem {
    fn thread_local_system(self) -> Box<dyn System>;
}

impl<F> IntoThreadLocalSystem for F
where
    F: ThreadLocalSystemFn,
{
    fn thread_local_system(mut self) -> Box<dyn System> {
        Box::new(SystemFn {
            state: (),
            thread_local_func: move |world, resources, _| {
                self.run(world, resources);
            },
            func: |_, _, _| {},
            init_func: |_, _, _| {},
            update_func: |_, _, _| {},
            thread_local_execution: ThreadLocalExecution::Immediate,
            name: core::any::type_name::<F>().into(),
            id: SystemId::new(),
            resource_access: TypeAccess::default(),
            archetype_component_access: TypeAccess::default(),
        })
    }
}

/// A thread local system function
pub trait ThreadLocalSystemFn: Send + Sync + 'static {
    fn run(&mut self, world: &mut World, resource: &mut Resources);
}

impl<F> ThreadLocalSystemFn for F
where
    F: FnMut(&mut World, &mut Resources) + Send + Sync + 'static,
{
    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        self(world, resources);
    }
}

#[cfg(test)]
mod tests {
    use super::{IntoForEachSystem, IntoQuerySystem, Query};
    use crate::{
        resource::{ResMut, Resources},
        schedule::Schedule,
        ChangedRes, Mut, QuerySet,
    };
    use bevy_hecs::{Entity, With, World};

    #[derive(Debug, Eq, PartialEq)]
    struct A;
    struct B;
    struct C;
    struct D;

    #[test]
    fn query_system_gets() {
        fn query_system(
            mut ran: ResMut<bool>,
            entity_query: Query<With<A, Entity>>,
            b_query: Query<&B>,
            a_c_query: Query<(&A, &C)>,
            d_query: Query<&D>,
        ) {
            let entities = entity_query.iter().collect::<Vec<Entity>>();
            assert!(
                b_query.get_component::<B>(entities[0]).is_err(),
                "entity 0 should not have B"
            );
            assert!(
                b_query.get_component::<B>(entities[1]).is_ok(),
                "entity 1 should have B"
            );
            assert!(
                b_query.get_component::<A>(entities[1]).is_err(),
                "entity 1 should have A, but b_query shouldn't have access to it"
            );
            assert!(
                b_query.get_component::<D>(entities[3]).is_err(),
                "entity 3 should have D, but it shouldn't be accessible from b_query"
            );
            assert!(
                b_query.get_component::<C>(entities[2]).is_err(),
                "entity 2 has C, but it shouldn't be accessible from b_query"
            );
            assert!(
                a_c_query.get_component::<C>(entities[2]).is_ok(),
                "entity 2 has C, and it should be accessible from a_c_query"
            );
            assert!(
                a_c_query.get_component::<D>(entities[3]).is_err(),
                "entity 3 should have D, but it shouldn't be accessible from b_query"
            );
            assert!(
                d_query.get_component::<D>(entities[3]).is_ok(),
                "entity 3 should have D"
            );

            *ran = true;
        }

        let mut world = World::default();
        let mut resources = Resources::default();
        resources.insert(false);
        world.spawn((A,));
        world.spawn((A, B));
        world.spawn((A, C));
        world.spawn((A, D));

        let mut schedule = Schedule::default();
        schedule.add_stage("update");
        schedule.add_system_to_stage("update", query_system.system());

        schedule.run(&mut world, &mut resources);

        assert!(*resources.get::<bool>().unwrap(), "system ran");
    }

    #[test]
    fn or_query_set_system() {
        // Regression test for issue #762
        use crate::{Added, Changed, Mutated, Or};
        fn query_system(
            mut ran: ResMut<bool>,
            set: QuerySet<(
                Query<Or<(Changed<A>, Changed<B>)>>,
                Query<Or<(Added<A>, Added<B>)>>,
                Query<Or<(Mutated<A>, Mutated<B>)>>,
            )>,
        ) {
            let changed = set.q0().iter().count();
            let added = set.q1().iter().count();
            let mutated = set.q2().iter().count();

            assert_eq!(changed, 1);
            assert_eq!(added, 1);
            assert_eq!(mutated, 0);

            *ran = true;
        }

        let mut world = World::default();
        let mut resources = Resources::default();
        resources.insert(false);
        world.spawn((A, B));

        let mut schedule = Schedule::default();
        schedule.add_stage("update");
        schedule.add_system_to_stage("update", query_system.system());

        schedule.run(&mut world, &mut resources);

        assert!(*resources.get::<bool>().unwrap(), "system ran");
    }

    #[test]
    fn changed_resource_system() {
        fn incr_e_on_flip(_run_on_flip: ChangedRes<bool>, mut i: Mut<i32>) {
            *i += 1;
        }

        let mut world = World::default();
        let mut resources = Resources::default();
        resources.insert(false);
        let ent = world.spawn((0,));

        let mut schedule = Schedule::default();
        schedule.add_stage("update");
        schedule.add_system_to_stage("update", incr_e_on_flip.system());

        schedule.run(&mut world, &mut resources);
        assert_eq!(*(world.get::<i32>(ent).unwrap()), 1);

        schedule.run(&mut world, &mut resources);
        assert_eq!(*(world.get::<i32>(ent).unwrap()), 1);

        *resources.get_mut::<bool>().unwrap() = true;
        schedule.run(&mut world, &mut resources);
        assert_eq!(*(world.get::<i32>(ent).unwrap()), 2);
    }

    #[test]
    #[should_panic]
    fn conflicting_query_mut_system() {
        fn sys(_q1: Query<&mut A>, _q2: Query<&mut A>) {}

        let mut world = World::default();
        let mut resources = Resources::default();
        world.spawn((A,));

        let mut schedule = Schedule::default();
        schedule.add_stage("update");
        schedule.add_system_to_stage("update", sys.system());

        schedule.run(&mut world, &mut resources);
    }

    #[test]
    #[should_panic]
    fn conflicting_query_immut_system() {
        fn sys(_q1: Query<&A>, _q2: Query<&mut A>) {}

        let mut world = World::default();
        let mut resources = Resources::default();
        world.spawn((A,));

        let mut schedule = Schedule::default();
        schedule.add_stage("update");
        schedule.add_system_to_stage("update", sys.system());

        schedule.run(&mut world, &mut resources);
    }

    #[test]
    fn query_set_system() {
        fn sys(_set: QuerySet<(Query<&mut A>, Query<&B>)>) {}

        let mut world = World::default();
        let mut resources = Resources::default();
        world.spawn((A,));

        let mut schedule = Schedule::default();
        schedule.add_stage("update");
        schedule.add_system_to_stage("update", sys.system());

        schedule.run(&mut world, &mut resources);
    }

    #[test]
    #[should_panic]
    fn conflicting_query_with_query_set_system() {
        fn sys(_query: Query<&mut A>, _set: QuerySet<(Query<&mut A>, Query<&B>)>) {}

        let mut world = World::default();
        let mut resources = Resources::default();
        world.spawn((A,));

        let mut schedule = Schedule::default();
        schedule.add_stage("update");
        schedule.add_system_to_stage("update", sys.system());

        schedule.run(&mut world, &mut resources);
    }

    #[test]
    #[should_panic]
    fn conflicting_query_sets_system() {
        fn sys(_set_1: QuerySet<(Query<&mut A>,)>, _set_2: QuerySet<(Query<&mut A>, Query<&B>)>) {}

        let mut world = World::default();
        let mut resources = Resources::default();
        world.spawn((A,));

        let mut schedule = Schedule::default();
        schedule.add_stage("update");
        schedule.add_system_to_stage("update", sys.system());

        schedule.run(&mut world, &mut resources);
    }
}
