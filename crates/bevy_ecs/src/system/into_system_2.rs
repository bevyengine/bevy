//! Create a system from a function

use std::marker::PhantomData;

use crate::{BoxedSystem, In, Resources, System, SystemId, SystemState, TypeAccess};

use super::system_param_2::{Local, ParamState, SystemParam};

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
pub struct FuncSystem<F, Input, Params: SystemParam> {
    param_state: Params::State,
    function: F,
    sys_state: SystemState,
    input: PhantomData<fn(Input)>,
}

impl<Params: SystemParam, F, Out: 'static> AsSystem for FuncSystemPrepare<F, (), Params>
where
    FuncSystem<F, (), Params>: System<In = (), Out = Out>,
{
    type System = FuncSystem<F, (), Params>;

    fn as_system(self, resources: &mut Resources) -> Self::System {
        FuncSystem {
            param_state: Params::create_state(self.config, resources),
            function: self.function,
            sys_state: SystemState {
                name: std::any::type_name::<F>().into(),
                archetype_component_access: TypeAccess::default(),
                resource_access: TypeAccess::default(),
                local_resource_access: TypeAccess::default(),
                id: SystemId::new(),
                commands: Default::default(),
                arc_commands: Default::default(),
                current_query_index: Default::default(),
                query_archetype_component_accesses: Vec::new(),
                query_accesses: Vec::new(),
                query_type_names: Vec::new(),
            },
            input: PhantomData,
        }
    }
}

impl<Input, Params: SystemParam, F, Out: 'static> AsSystem
    for FuncSystemPrepare<F, In<Input>, Params>
where
    FuncSystem<F, In<Input>, Params>: System<In = Input, Out = Out>,
{
    type System = FuncSystem<F, In<Input>, Params>;

    fn as_system(self, resources: &mut Resources) -> Self::System {
        FuncSystem {
            param_state: Params::create_state(self.config, resources),
            function: self.function,
            sys_state: SystemState {
                name: std::any::type_name::<F>().into(),
                archetype_component_access: TypeAccess::default(),
                resource_access: TypeAccess::default(),
                local_resource_access: TypeAccess::default(),
                id: SystemId::new(),
                commands: Default::default(),
                arc_commands: Default::default(),
                current_query_index: Default::default(),
                query_archetype_component_accesses: Vec::new(),
                query_accesses: Vec::new(),
                query_type_names: Vec::new(),
            },
            input: PhantomData,
        }
    }
}

impl<F, Input, Out, P: SystemParam> IntoSystem<In<Input>, (P,)> for F
where
    F: FnMut(In<Input>, P) -> Out
        + FnMut(In<Input>, <<P as SystemParam>::State as ParamState>::Item) -> Out,
{
    type SystemConfig = FuncSystemPrepare<F, In<Input>, (P,)>;

    fn system(self) -> Self::SystemConfig {
        FuncSystemPrepare {
            function: self,
            config: (P::default_config(),),
            input: PhantomData,
        }
    }
}

impl<F, P: SystemParam, Out> IntoSystem<(), (P,)> for F
where
    F: FnMut(P) -> Out + FnMut(<<P as SystemParam>::State as ParamState>::Item) -> Out,
{
    type SystemConfig = FuncSystemPrepare<F, (), (P,)>;

    fn system(self) -> Self::SystemConfig {
        FuncSystemPrepare {
            function: self,
            config: (P::default_config(),),
            input: PhantomData,
        }
    }
}

#[allow(non_snake_case)]
impl<F, P: SystemParam + 'static, Out: 'static> System for FuncSystem<F, (), (P,)>
where
    F: Send + Sync + 'static + FnMut(<<P as SystemParam>::State as ParamState>::Item) -> Out,
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
        let (P,) = &mut self.param_state;
        Some((self.function)(P.get_param(
            &self.sys_state,
            world,
            resources,
        )?))
    }

    fn run_thread_local(&mut self, world: &mut crate::World, resources: &mut Resources) {
        let (P,) = &mut self.param_state;
        P.run_sync(world, resources)
    }

    fn initialize(&mut self, world: &mut crate::World, resources: &mut Resources) {
        // This code can be easily macro generated
        let (P,) = &mut self.param_state;
        P.init(&mut self.sys_state, world, resources)
    }
}

#[allow(non_snake_case)]
impl<F, Input: 'static, P: SystemParam + 'static, Out: 'static> System
    for FuncSystem<F, In<Input>, (P,)>
where
    F: Send
        + Sync
        + 'static
        + FnMut(In<Input>, <<P as SystemParam>::State as ParamState>::Item) -> Out,
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
        let (P,) = &mut self.param_state;
        Some((self.function)(
            In(input),
            P.get_param(&self.sys_state, world, resources)?,
        ))
    }

    fn run_thread_local(&mut self, world: &mut crate::World, resources: &mut Resources) {
        let (P,) = &mut self.param_state;
        P.run_sync(world, resources)
    }

    fn initialize(&mut self, world: &mut crate::World, resources: &mut Resources) {
        // This code can be easily macro generated
        let (P,) = &mut self.param_state;
        P.init(&mut self.sys_state, world, resources)
    }
}

/* impl_into_system!();
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

// We can't use default because these use more types than tuples
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);
 */

fn test_system(In(input): In<u32>, local: &mut Local<u32>) {
    **local = input;
}

fn test_it() {
    test_accept(test_system.system().configure(|it| it.0 = Some(32)));
}

fn test_accept<C: AsSystem>(it: C)
where
    C::System: System<In = u32, Out = ()>,
{
    let mut res = Resources::default();
    let sys = Box::new(it.as_system(&mut res));
    test_accept_boxed(sys);
}

fn test_accept_boxed(it: BoxedSystem<u32>) {
    it.id();
}
