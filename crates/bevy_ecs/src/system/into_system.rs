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
    fn as_system(self, resources: &mut Resources) -> Self::System;
}

/// Every system can trivially be converted into a System
impl<T: System> AsSystem for T {
    type System = T;

    fn as_system(self, _: &mut Resources) -> Self::System {
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

    fn as_system(self, resources: &mut Resources) -> Self::System {
        FuncSystem {
            param_state: Params::create_state(self.config, resources),
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

    fn as_system(self, resources: &mut Resources) -> Self::System {
        FuncSystem {
            param_state: Params::create_state(self.config, resources),
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
/*
impl<F, Input, Out, T1: SystemParam> IntoSystem<In<Input>, (T1,)> for F
where
    F: FnMut(In<Input>, T1) -> Out
        + FnMut(In<Input>, <<T1 as SystemParam>::State as ParamState>::Item) -> Out,
{
    type SystemConfig = FuncSystemPrepare<F, In<Input>, (T1,)>;
    fn system(self) -> Self::SystemConfig {
        FuncSystemPrepare {
            function: self,
            config: (T1::default_config(),),
            input: PhantomData,
        }
    }
}
impl<F, Out, T1: SystemParam> IntoSystem<(), (T1,)> for F
where
    F: FnMut(T1) -> Out + FnMut(<<T1 as SystemParam>::State as ParamState>::Item) -> Out,
{
    type SystemConfig = FuncSystemPrepare<F, (), (T1,)>;
    fn system(self) -> Self::SystemConfig {
        FuncSystemPrepare {
            function: self,
            config: (T1::default_config(),),
            input: PhantomData,
        }
    }
}
#[allow(non_snake_case)]
#[allow(unused_variables)]
impl<F, T1: for<'a> ParamState<'a>, Out: 'static> System for FuncSystem<F, (), (T1,)>
where
    F: Send + Sync + 'static + FnMut(<T1 as ParamState>::Item) -> Out,
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
        let (T1,) = &mut self.param_state;
        Some((self.function)(T1.get_param(
            &self.sys_state,
            world,
            resources,
        )?))
    }
    fn run_thread_local(&mut self, world: &mut crate::World, resources: &mut Resources) {
        let (T1,) = &mut self.param_state;
        T1.run_sync(world, resources);
    }
    fn initialize(&mut self, world: &mut crate::World, resources: &mut Resources) {
        let (T1,) = &mut self.param_state;
        T1.init(&mut self.sys_state, world, resources);
    }
}
 */
