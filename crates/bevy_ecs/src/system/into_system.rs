//! Create a system from a function

use std::{any::TypeId, borrow::Cow, cell::UnsafeCell, marker::PhantomData};

use crate::{ArchetypeComponent, QueryAccess, Resources, System, SystemId, TypeAccess, World};

use super::system_param::{ParamState, SystemParam};

pub struct SystemState {
    pub(crate) id: SystemId,
    pub(crate) name: Cow<'static, str>,
    pub(crate) archetype_component_access: TypeAccess<ArchetypeComponent>,
    pub(crate) resource_access: TypeAccess<TypeId>,
    pub(crate) query_archetype_component_accesses: Vec<TypeAccess<ArchetypeComponent>>,
    pub(crate) query_accesses: Vec<Vec<QueryAccess>>,
    pub(crate) query_type_names: Vec<&'static str>,
    pub(crate) current_query_index: UnsafeCell<usize>,
}

// SAFE: UnsafeCell<Commands> and UnsafeCell<usize> only accessed from the thread they are scheduled on
unsafe impl Sync for SystemState {}

impl SystemState {
    pub fn reset_indices(&mut self) {
        // SAFE: done with unique mutable access to Self
        unsafe {
            *self.current_query_index.get() = 0;
        }
    }

    pub fn update(&mut self, world: &World) {
        self.archetype_component_access.clear();
        let mut conflict_index = None;
        let mut conflict_name = None;
        for (i, (query_accesses, component_access)) in self
            .query_accesses
            .iter()
            .zip(self.query_archetype_component_accesses.iter_mut())
            .enumerate()
        {
            component_access.clear();
            for query_access in query_accesses.iter() {
                query_access.get_world_archetype_access(world, Some(component_access));
            }
            if !component_access.is_compatible(&self.archetype_component_access) {
                conflict_index = Some(i);
                conflict_name = component_access
                    .get_conflict(&self.archetype_component_access)
                    .and_then(|archetype_component| {
                        query_accesses
                            .iter()
                            .filter_map(|query_access| {
                                query_access.get_type_name(archetype_component.component)
                            })
                            .next()
                    });
                break;
            }
            self.archetype_component_access.union(component_access);
        }
        if let Some(conflict_index) = conflict_index {
            let mut conflicts_with_index = None;
            for prior_index in 0..conflict_index {
                if !self.query_archetype_component_accesses[conflict_index]
                    .is_compatible(&self.query_archetype_component_accesses[prior_index])
                {
                    conflicts_with_index = Some(prior_index);
                }
            }
            panic!("System {} has conflicting queries. {} conflicts with the component access [{}] in this prior query: {}.",
                self.name,
                self.query_type_names[conflict_index],
                conflict_name.unwrap_or("Unknown"),
                conflicts_with_index.map(|index| self.query_type_names[index]).unwrap_or("Unknown"));
        }
    }
}

/// A type which can be converted into a system at some point in the future
/// This should only be implemented for functions
/// ```rust
/// use bevy_ecs::{IntoSystem, Local};
/// fn legal_system(it: Local<u32>){}
/// # fn main(){
/// legal_system.system();
/// # }
/// ```
/// This will not compile, since it is trying to get a 'static reference to the value
/// ```compile_fail
/// use bevy_ecs::{IntoSystem, Local};
/// fn illegal_system(it: Local<'static, u32>){}
/// # fn main(){
/// illegal_system.system();
/// # }
/// ```
// TODO: Seal?
pub trait IntoSystem<Input, Params> {
    type SystemConfig;
    fn system(self) -> Self::SystemConfig;
}

/// A function which will be turned into a system and the configuration for its state
/// You should call [`FuncSystemPrepare::configure`] to modify the configuration using a builder pattern
pub struct FuncSystemPrepare<Func, Input, Params: SystemParam> {
    function: Func,
    pub config: Params::Config,
    input: PhantomData<fn(Input)>,
}

impl<F, P: SystemParam, I> FuncSystemPrepare<F, I, P> {
    /// Modify the configuration of self using a builder pattern. This allows setting the value for local values. For example:
    // TODO: Actually insert that example
    /// ```
    /// /* use bevy_ecs::Local;
    /// fn my_system(){} */
    /// ```
    pub fn configure(mut self, f: impl FnOnce(&mut <P as SystemParam>::Config)) -> Self {
        f(&mut self.config);
        self
    }
}

/// Convert a type into an actual system. This is not to be confused with [`IntoSystem`],
/// which creates a value which itself implements [`AsSystem`]
/// Bounds on the input type of any method which wishes to accept any system should be expressed in terms of this trait
pub trait AsSystem {
    type System: System;
    fn as_system(self) -> Self::System;
}

/// Every system can trivially be converted into a System
impl<T: System> AsSystem for T {
    type System = T;

    fn as_system(self) -> Self::System {
        self
    }
}

/// A system which runs based on the given function
pub struct FuncSystem<F, Input, State> {
    param_state: State,
    function: F,
    sys_state: SystemState,
    input: PhantomData<fn(Input)>,
}

impl<Params: SystemParam, F, Out: 'static> AsSystem for FuncSystemPrepare<F, (), Params>
where
    FuncSystem<F, (), Params::State>: System<In = (), Out = Out>,
{
    type System = FuncSystem<F, (), Params::State>;

    fn as_system(self) -> Self::System {
        FuncSystem {
            param_state: Params::create_state(self.config),
            function: self.function,
            sys_state: SystemState {
                name: std::any::type_name::<F>().into(),
                archetype_component_access: TypeAccess::default(),
                resource_access: TypeAccess::default(),
                id: SystemId::new(),
                current_query_index: Default::default(),
                query_archetype_component_accesses: Vec::new(),
                query_accesses: Vec::new(),
                query_type_names: Vec::new(),
            },
            input: PhantomData,
        }
    }
}

pub struct In<In>(pub In);

impl<Input, Params: SystemParam, F, Out: 'static> AsSystem
    for FuncSystemPrepare<F, In<Input>, Params>
where
    FuncSystem<F, In<Input>, Params::State>: System<In = Input, Out = Out>,
{
    type System = FuncSystem<F, In<Input>, Params::State>;

    fn as_system(self) -> Self::System {
        FuncSystem {
            param_state: Params::create_state(self.config),
            function: self.function,
            sys_state: SystemState {
                name: std::any::type_name::<F>().into(),
                archetype_component_access: TypeAccess::default(),
                resource_access: TypeAccess::default(),
                id: SystemId::new(),
                current_query_index: Default::default(),
                query_archetype_component_accesses: Vec::new(),
                query_accesses: Vec::new(),
                query_type_names: Vec::new(),
            },
            input: PhantomData,
        }
    }
}

macro_rules! impl_into_system {
    ($($param: ident),*) => {
        impl<F, Input, Out, $($param: SystemParam,)*> IntoSystem<In<Input>, ($($param,)*)> for F
        where
            F: FnMut(In<Input>, $($param,)*) -> Out
                + FnMut(In<Input>, $(<<$param as SystemParam>::State as ParamState>::Item,)*) -> Out,
        {
            type SystemConfig = FuncSystemPrepare<F, In<Input>, ($($param,)*)>;

            fn system(self) -> Self::SystemConfig {
                FuncSystemPrepare {
                    function: self,
                    config: ($($param::default_config(),)*),
                    input: PhantomData,
                }
            }
        }

        impl<F, Out, $($param: SystemParam,)*> IntoSystem<(), ($($param,)*)> for F
        where
            F: FnMut( $($param,)*) -> Out
                + FnMut($(<<$param as SystemParam>::State as ParamState>::Item,)*) -> Out,
        {
            type SystemConfig = FuncSystemPrepare<F, (), ($($param,)*)>;

            fn system(self) -> Self::SystemConfig {
                FuncSystemPrepare {
                    function: self,
                    config: ($($param::default_config(),)*),
                    input: PhantomData,
                }
            }
        }


        #[allow(non_snake_case)]
        #[allow(unused_variables)] // For the zero item case
        impl<F, $($param: for<'a> ParamState<'a>,)* Out: 'static> System for FuncSystem<F, (), ($($param,)*)>
        where
            F: Send
                + Sync
                + 'static
                + FnMut($(<$param as ParamState>::Item,)*) -> Out,
        {
            type In = ();
            type Out = Out;

            fn name(&self) -> std::borrow::Cow<'static, str> {
                self.sys_state.name.clone()
            }

            fn id(&self) -> SystemId {
                self.sys_state.id
            }

            fn update(&mut self, world: &crate::World) {
                self.sys_state.update(world)
            }

            fn archetype_component_access(&self) -> &crate::TypeAccess<crate::ArchetypeComponent> {
                &self.sys_state.archetype_component_access
            }

            fn resource_access(&self) -> &crate::TypeAccess<std::any::TypeId> {
                &self.sys_state.resource_access
            }

            fn thread_local_execution(&self) -> crate::ThreadLocalExecution {
                crate::ThreadLocalExecution::NextFlush
            }

            unsafe fn run_unsafe(
                &mut self,
                _: Self::In,
                world: &crate::World,
                resources: &Resources,
            ) -> Option<Self::Out> {
                self.sys_state.reset_indices();
                let ($($param,)*) = &mut self.param_state;
                Some((self.function)($($param.get_param(
                    &self.sys_state,
                    world,
                    resources,
                )?,)*))
            }

            fn run_thread_local(&mut self, world: &mut crate::World, resources: &mut Resources) {
                let ($($param,)*) = &mut self.param_state;
                $($param.run_sync(world, resources);)*

            }

            fn initialize(&mut self, world: &mut crate::World, resources: &mut Resources) {
                // This code can be easily macro generated
                let ($($param,)*) = &mut self.param_state;
                $($param.init(&mut self.sys_state, world, resources);)*
            }
        }

        #[allow(non_snake_case)]
        #[allow(unused_variables)] // For the zero item case
        impl<F, Input: 'static, $($param: for<'a> ParamState<'a>,)* Out: 'static> System
            for FuncSystem<F, In<Input>, ($($param,)*)>
        where
            F: Send
                + Sync
                + 'static
                + FnMut(In<Input>, $(<$param as ParamState>::Item,)*) -> Out,
        {
            type In = Input;
            type Out = Out;

            fn name(&self) -> std::borrow::Cow<'static, str> {
                self.sys_state.name.clone()
            }

            fn id(&self) -> SystemId {
                self.sys_state.id
            }

            fn update(&mut self, world: &crate::World) {
                self.sys_state.update(world)
            }

            fn archetype_component_access(&self) -> &crate::TypeAccess<crate::ArchetypeComponent> {
                &self.sys_state.archetype_component_access
            }

            fn resource_access(&self) -> &crate::TypeAccess<std::any::TypeId> {
                &self.sys_state.resource_access
            }

            fn thread_local_execution(&self) -> crate::ThreadLocalExecution {
                crate::ThreadLocalExecution::NextFlush
            }

            unsafe fn run_unsafe(
                &mut self,
                input: Self::In,
                world: &crate::World,
                resources: &Resources,
            ) -> Option<Self::Out> {
                self.sys_state.reset_indices();
                let ($($param,)*) = &mut self.param_state;
                Some((self.function)(In(input),
                    $($param.get_param(
                    &self.sys_state,
                    world,
                    resources,
                )?,)*))
            }

            fn run_thread_local(&mut self, world: &mut crate::World, resources: &mut Resources) {
                let ($($param,)*) = &mut self.param_state;
                $($param.run_sync(world, resources);)*

            }

            fn initialize(&mut self, world: &mut crate::World, resources: &mut Resources) {
                // This code can be easily macro generated
                let ($($param,)*) = &mut self.param_state;
                $($param.init(&mut self.sys_state, world, resources);)*
            }
        }
    };
}

impl_into_system!();
impl_into_system!(T1);
impl_into_system!(T1, T2);
impl_into_system!(T1, T2, T3);
impl_into_system!(T1, T2, T3, T4);
impl_into_system!(T1, T2, T3, T4, T5);
impl_into_system!(T1, T2, T3, T4, T5, T6);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);

#[cfg(test)]
mod tests {
    use super::IntoSystem;
    use crate::{
        clear_trackers_system,
        resource::{Res, ResMut, Resources},
        schedule::Schedule,
        AsSystem, ChangedRes, Entity, Local, Or, Query, QuerySet, System, SystemStage, With, World,
    };

    #[derive(Debug, Eq, PartialEq, Default)]
    struct A;
    struct B;
    struct C;
    struct D;

    #[test]
    fn query_system_gets() {
        fn query_system(
            mut ran: ResMut<bool>,
            entity_query: Query<Entity, With<A>>,
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

        run_system(&mut world, &mut resources, query_system.system());

        assert!(*resources.get::<bool>().unwrap(), "system ran");
    }

    #[test]
    fn or_query_set_system() {
        // Regression test for issue #762
        use crate::{Added, Changed, Mutated, Or};
        fn query_system(
            mut ran: ResMut<bool>,
            set: QuerySet<(
                Query<(), Or<(Changed<A>, Changed<B>)>>,
                Query<(), Or<(Added<A>, Added<B>)>>,
                Query<(), Or<(Mutated<A>, Mutated<B>)>>,
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

        run_system(&mut world, &mut resources, query_system.system());

        assert!(*resources.get::<bool>().unwrap(), "system ran");
    }

    #[test]
    fn changed_resource_system() {
        fn incr_e_on_flip(_run_on_flip: ChangedRes<bool>, mut query: Query<&mut i32>) {
            for mut i in query.iter_mut() {
                *i += 1;
            }
        }

        let mut world = World::default();
        let mut resources = Resources::default();
        resources.insert(false);
        let ent = world.spawn((0,));

        let mut schedule = Schedule::default();
        let mut update = SystemStage::parallel();
        update.add_system(incr_e_on_flip.system());
        schedule.add_stage("update", update);
        schedule.add_stage(
            "clear_trackers",
            SystemStage::single(clear_trackers_system.system()),
        );

        schedule.initialize_and_run(&mut world, &mut resources);
        assert_eq!(*(world.get::<i32>(ent).unwrap()), 1);

        schedule.initialize_and_run(&mut world, &mut resources);
        assert_eq!(*(world.get::<i32>(ent).unwrap()), 1);

        *resources.get_mut::<bool>().unwrap() = true;
        schedule.initialize_and_run(&mut world, &mut resources);
        assert_eq!(*(world.get::<i32>(ent).unwrap()), 2);
    }

    #[test]
    fn changed_resource_or_system() {
        fn incr_e_on_flip(
            _or: Or<(Option<ChangedRes<bool>>, Option<ChangedRes<i32>>)>,
            mut query: Query<&mut i32>,
        ) {
            for mut i in query.iter_mut() {
                *i += 1;
            }
        }

        let mut world = World::default();
        let mut resources = Resources::default();
        resources.insert(false);
        resources.insert::<i32>(10);
        let ent = world.spawn((0,));

        let mut schedule = Schedule::default();
        let mut update = SystemStage::parallel();
        update.add_system(incr_e_on_flip.system());
        schedule.add_stage("update", update);
        schedule.add_stage(
            "clear_trackers",
            SystemStage::single(clear_trackers_system.system()),
        );

        schedule.initialize_and_run(&mut world, &mut resources);
        assert_eq!(*(world.get::<i32>(ent).unwrap()), 1);

        schedule.initialize_and_run(&mut world, &mut resources);
        assert_eq!(*(world.get::<i32>(ent).unwrap()), 1);

        *resources.get_mut::<bool>().unwrap() = true;
        schedule.initialize_and_run(&mut world, &mut resources);
        assert_eq!(*(world.get::<i32>(ent).unwrap()), 2);

        schedule.initialize_and_run(&mut world, &mut resources);
        assert_eq!(*(world.get::<i32>(ent).unwrap()), 2);

        *resources.get_mut::<i32>().unwrap() = 20;
        schedule.initialize_and_run(&mut world, &mut resources);
        assert_eq!(*(world.get::<i32>(ent).unwrap()), 3);
    }

    #[test]
    #[should_panic]
    fn conflicting_query_mut_system() {
        fn sys(_q1: Query<&mut A>, _q2: Query<&mut A>) {}

        let mut world = World::default();
        let mut resources = Resources::default();
        world.spawn((A,));

        run_system(&mut world, &mut resources, sys.system());
    }

    #[test]
    #[should_panic]
    fn conflicting_query_immut_system() {
        fn sys(_q1: Query<&A>, _q2: Query<&mut A>) {}

        let mut world = World::default();
        let mut resources = Resources::default();
        world.spawn((A,));

        run_system(&mut world, &mut resources, sys.system());
    }

    #[test]
    fn query_set_system() {
        fn sys(_set: QuerySet<(Query<&mut A>, Query<&B>)>) {}

        let mut world = World::default();
        let mut resources = Resources::default();
        world.spawn((A,));

        run_system(&mut world, &mut resources, sys.system());
    }

    #[test]
    #[should_panic]
    fn conflicting_query_with_query_set_system() {
        fn sys(_query: Query<&mut A>, _set: QuerySet<(Query<&mut A>, Query<&B>)>) {}

        let mut world = World::default();
        let mut resources = Resources::default();
        world.spawn((A,));

        run_system(&mut world, &mut resources, sys.system());
    }

    #[test]
    #[should_panic]
    fn conflicting_query_sets_system() {
        fn sys(_set_1: QuerySet<(Query<&mut A>,)>, _set_2: QuerySet<(Query<&mut A>, Query<&B>)>) {}

        let mut world = World::default();
        let mut resources = Resources::default();
        world.spawn((A,));
        run_system(&mut world, &mut resources, sys.system());
    }

    fn run_system<S: AsSystem>(world: &mut World, resources: &mut Resources, system: S)
    where
        S::System: System<In = (), Out = ()>,
    {
        let mut schedule = Schedule::default();
        let mut update = SystemStage::parallel();
        update.add_system(system);
        schedule.add_stage("update", update);
        schedule.initialize_and_run(world, resources);
    }

    #[derive(Default)]
    struct BufferRes {
        _buffer: Vec<u8>,
    }

    fn test_for_conflicting_resources<S: AsSystem>(sys: S)
    where
        S::System: System<In = (), Out = ()>,
    {
        let mut world = World::default();
        let mut resources = Resources::default();
        resources.insert(BufferRes::default());
        resources.insert(A);
        resources.insert(B);
        run_system(&mut world, &mut resources, sys.as_system());
    }

    #[test]
    #[should_panic]
    fn conflicting_system_resources() {
        fn sys(_: ResMut<BufferRes>, _: Res<BufferRes>) {}
        test_for_conflicting_resources(sys.system())
    }

    #[test]
    #[should_panic]
    fn conflicting_system_resources_reverse_order() {
        fn sys(_: Res<BufferRes>, _: ResMut<BufferRes>) {}
        test_for_conflicting_resources(sys.system())
    }

    #[test]
    #[should_panic]
    fn conflicting_system_resources_multiple_mutable() {
        fn sys(_: ResMut<BufferRes>, _: ResMut<BufferRes>) {}
        test_for_conflicting_resources(sys.system())
    }

    #[test]
    #[should_panic]
    fn conflicting_changed_and_mutable_resource() {
        // A tempting pattern, but unsound if allowed.
        fn sys(_: ResMut<BufferRes>, _: ChangedRes<BufferRes>) {}
        test_for_conflicting_resources(sys.system())
    }

    #[test]
    fn nonconflicting_system_resources() {
        fn sys(_: Local<BufferRes>, _: ResMut<BufferRes>, _: Local<A>, _: ResMut<A>) {}
        test_for_conflicting_resources(sys.system())
    }
}
