pub use super::Query;
use super::TypeAccess;
use crate::{
    resource::{FetchResource, ResourceQuery, Resources, UnsafeClone},
    system::{ArchetypeAccess, Commands, System, SystemId, ThreadLocalExecution},
};
use bevy_hecs::{Fetch, Query as HecsQuery, World};
use std::borrow::Cow;

#[derive(Debug)]
pub(crate) struct SystemFn<State, F, ThreadLocalF, Init, SetArchetypeAccess>
where
    F: FnMut(&World, &Resources, &ArchetypeAccess, &mut State) + Send + Sync,
    ThreadLocalF: FnMut(&mut World, &mut Resources, &mut State) + Send + Sync,
    Init: FnMut(&mut World, &mut Resources, &mut State) + Send + Sync,
    SetArchetypeAccess: FnMut(&World, &mut ArchetypeAccess, &mut State) + Send + Sync,
    State: Send + Sync,
{
    pub state: State,
    pub func: F,
    pub thread_local_func: ThreadLocalF,
    pub init_func: Init,
    pub thread_local_execution: ThreadLocalExecution,
    pub resource_access: TypeAccess,
    pub name: Cow<'static, str>,
    pub id: SystemId,
    pub archetype_access: ArchetypeAccess,
    pub set_archetype_access: SetArchetypeAccess,
}

impl<State, F, ThreadLocalF, Init, SetArchetypeAccess> System
    for SystemFn<State, F, ThreadLocalF, Init, SetArchetypeAccess>
where
    F: FnMut(&World, &Resources, &ArchetypeAccess, &mut State) + Send + Sync,
    ThreadLocalF: FnMut(&mut World, &mut Resources, &mut State) + Send + Sync,
    Init: FnMut(&mut World, &mut Resources, &mut State) + Send + Sync,
    SetArchetypeAccess: FnMut(&World, &mut ArchetypeAccess, &mut State) + Send + Sync,
    State: Send + Sync,
{
    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn update_archetype_access(&mut self, world: &World) {
        (self.set_archetype_access)(world, &mut self.archetype_access, &mut self.state);
    }

    fn archetype_access(&self) -> &ArchetypeAccess {
        &self.archetype_access
    }

    fn resource_access(&self) -> &TypeAccess {
        &self.resource_access
    }

    fn thread_local_execution(&self) -> ThreadLocalExecution {
        self.thread_local_execution
    }

    #[inline]
    fn run(&mut self, world: &World, resources: &Resources) {
        (self.func)(world, resources, &self.archetype_access, &mut self.state);
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
                    state: Commands::default(),
                    thread_local_execution: ThreadLocalExecution::NextFlush,
                    name: core::any::type_name::<Self>().into(),
                    id,
                    func: move |world, resources, _archetype_access, state| {
                        {
                            if let Some(($($resource,)*)) = resources.query_system::<($($resource,)*)>(id) {
                                // SAFE: the scheduler has ensured that there is no archetype clashing here
                                unsafe {
                                    for ($($component,)*) in world.query_unchecked::<($($component,)*)>().iter() {
                                        fn_call!(self, ($($commands, state)*), ($($resource),*), ($($component),*))
                                    }
                                }
                            }
                        }
                    },
                    thread_local_func: move |world, resources, state| {
                        state.apply(world, resources);
                    },
                    init_func: move |world, resources, state| {
                        <($($resource,)*)>::initialize(resources, Some(id));
                        state.set_entity_reserver(world.get_entity_reserver())
                    },
                    resource_access: <<($($resource,)*) as ResourceQuery>::Fetch as FetchResource>::access(),
                    archetype_access: ArchetypeAccess::default(),
                    set_archetype_access: |world, archetype_access, _state| {
                        archetype_access.clear();
                        archetype_access.set_access_for_query::<($($component,)*)>(world);
                    },
                })
            }
        }
    };
}

struct QuerySystemState {
    archetype_accesses: Vec<ArchetypeAccess>,
    commands: Commands,
}

/// Converts `Self` into a Query System
pub trait IntoQuerySystem<Commands, R, Q> {
    fn system(self) -> Box<dyn System>;
}

macro_rules! impl_into_query_system {
    (($($commands: ident)*), ($($resource: ident),*), ($($query: ident),*)) => {
        impl<Func, $($resource,)* $($query,)*> IntoQuerySystem<($($commands,)*), ($($resource,)*), ($($query,)*)> for Func where
            Func:
                FnMut($($commands,)* $($resource,)* $(Query<$query>,)*) +
                FnMut(
                    $($commands,)*
                    $(<<$resource as ResourceQuery>::Fetch as FetchResource>::Item,)*
                    $(Query<$query>,)*) +
                Send + Sync +'static,
            $($query: HecsQuery,)*
            $($resource: ResourceQuery,)*
        {
            #[allow(non_snake_case)]
            #[allow(unused_variables)]
            #[allow(unused_unsafe)]
            #[allow(unused_assignments)]
            #[allow(unused_mut)]
            fn system(mut self) -> Box<dyn System> {
                let id = SystemId::new();
                $(let $query = ArchetypeAccess::default();)*
                Box::new(SystemFn {
                    state: QuerySystemState {
                        archetype_accesses: vec![
                            $($query,)*
                        ],
                        commands: Commands::default(),
                    },
                    thread_local_execution: ThreadLocalExecution::NextFlush,
                    id,
                    name: core::any::type_name::<Self>().into(),
                    func: move |world, resources, archetype_access, state| {
                        {
                            if let Some(($($resource,)*)) = resources.query_system::<($($resource,)*)>(id) {
                                let mut i = 0;
                                $(
                                    let $query = Query::<$query>::new(world, &state.archetype_accesses[i]);
                                    i += 1;
                                )*

                                let commands = &state.commands;
                                fn_call!(self, ($($commands, commands)*), ($($resource),*), ($($query),*))
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
                    archetype_access: ArchetypeAccess::default(),
                    set_archetype_access: |world, archetype_access, state| {
                        archetype_access.clear();
                        let mut i = 0;
                        let mut access: &mut ArchetypeAccess;
                        $(
                            access = &mut state.archetype_accesses[i];
                            access.clear();
                            access.set_access_for_query::<$query>(world);
                            archetype_access.union(access);
                            i += 1;
                         )*
                    },
                })
            }
        }
    };
}

macro_rules! fn_call {
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
        impl_into_query_system!((), ($($resource),*), ($($query),*));
        #[rustfmt::skip]
        impl_into_query_system!((Commands), ($($resource),*), ($($query),*));
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
            func: |_, _, _, _| {},
            init_func: |_, _, _| {},
            set_archetype_access: |_, _, _| {},
            thread_local_execution: ThreadLocalExecution::Immediate,
            name: core::any::type_name::<F>().into(),
            id: SystemId::new(),
            resource_access: TypeAccess::default(),
            archetype_access: ArchetypeAccess::default(),
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
        ChangedRes, Mut,
    };
    use bevy_hecs::{Entity, With, World};

    struct A;
    struct B;
    struct C;
    struct D;

    #[test]
    fn query_system_gets() {
        fn query_system(
            mut ran: ResMut<bool>,
            mut entity_query: Query<With<A, Entity>>,
            b_query: Query<&B>,
            a_c_query: Query<(&A, &C)>,
            d_query: Query<&D>,
        ) {
            let entities = entity_query.iter().iter().collect::<Vec<Entity>>();
            assert!(
                b_query.get::<B>(entities[0]).is_err(),
                "entity 0 should not have B"
            );
            assert!(
                b_query.get::<B>(entities[1]).is_ok(),
                "entity 1 should have B"
            );
            assert!(
                b_query.get::<A>(entities[1]).is_ok(),
                "entity 1 should have A, and it should (unintuitively) be accessible from b_query because b_query grabs read access to the (A,B) archetype");
            assert!(
                b_query.get::<D>(entities[3]).is_err(),
                "entity 3 should have D, but it shouldn't be accessible from b_query"
            );
            assert!(
                b_query.get::<C>(entities[2]).is_err(),
                "entity 2 has C, but it shouldn't be accessible from b_query"
            );
            assert!(
                a_c_query.get::<C>(entities[2]).is_ok(),
                "entity 2 has C, and it should be accessible from a_c_query"
            );
            assert!(
                a_c_query.get::<D>(entities[3]).is_err(),
                "entity 3 should have D, but it shouldn't be accessible from b_query"
            );
            assert!(
                d_query.get::<D>(entities[3]).is_ok(),
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
}
