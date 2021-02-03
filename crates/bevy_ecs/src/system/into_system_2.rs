use crate::{BoxedSystem, Resources, System, SystemId, SystemState, TypeAccess};

use super::system_param_2::{Local, ParamState, SystemParam};

pub struct FuncSystemPrepare<Func, Params: ParamList> {
    function: Func,
    config: Params::Config,
}
pub trait ParamList {
    type Config;
    type State: Send + Sync;
}

impl<P: SystemParam> ParamList for (P,) {
    type Config = (P::Config,);
    type State = (P::State,);
}

pub trait IntoSystem<Params> {
    type SystemConfig;
    fn system(self) -> Self::SystemConfig;
}

impl<F, P: SystemParam, Out> IntoSystem<(P,)> for F
where
    F: FnMut(P) -> Out + FnMut(<<P as SystemParam>::State as ParamState>::Item) -> Out,
{
    type SystemConfig = FuncSystemPrepare<F, (P,)>;

    fn system(self) -> Self::SystemConfig {
        FuncSystemPrepare {
            function: self,
            config: (P::Config::default(),),
        }
    }
}

impl<F, P: SystemParam, Out> FuncSystemPrepare<F, (P,)>
where
    F: FnMut(P) -> Out,
{
    pub fn configure(mut self, f: impl FnOnce(&mut <(P,) as ParamList>::Config)) -> Self {
        f(&mut self.config);
        self
    }
}

pub trait AsSystem {
    type In;
    type Out;
    type System: System<In = Self::In, Out = Self::Out>;
    fn as_system(self, resources: &mut Resources) -> Self::System;
}

pub struct FuncSystem<F, Params: ParamList> {
    param_state: Params::State,
    function: F,
    sys_state: SystemState,
}

impl<P: SystemParam, F, Out: 'static> AsSystem for FuncSystemPrepare<F, (P,)>
where
    FuncSystem<F, (P,)>: System<In = (), Out = Out>,
{
    type System = FuncSystem<F, (P,)>;
    type In = ();
    type Out = Out;

    fn as_system(self, resources: &mut Resources) -> Self::System {
        let (p_config,) = self.config;
        FuncSystem {
            function: self.function,
            param_state: (P::create_state(p_config, resources),),
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
        }
    }
}

#[allow(unused_variables)]
#[allow(unused_unsafe)]
#[allow(non_snake_case)]
impl<F, P: SystemParam + 'static, Out: 'static> System for FuncSystem<F, (P,)>
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
        input: Self::In,
        world: &crate::World,
        resources: &Resources,
    ) -> Option<Self::Out> {
        let (P,) = &mut self.param_state;
        Some((self.function)(P.get_param(
            &self.sys_state,
            world,
            resources,
        )))
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
    //
}

fn test_system(local: &mut Local<u32>) {
    **local = 15;
}

fn test_it() {
    test_accept(test_system.system().configure(|it| it.0 = Some(32)));
}

fn test_accept<C: AsSystem<In = (), Out = ()>>(it: C) {
    let mut res = Resources::default();
    let sys = Box::new(it.as_system(&mut res));
    test_accept_boxed(sys);
}

fn test_accept_boxed(it: BoxedSystem) {
    it.id();
}
