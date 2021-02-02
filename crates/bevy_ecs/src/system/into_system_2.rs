use crate::{Resources, System, SystemId};

use super::system_param_2::{Local, ParamState, SystemParam};

pub struct FuncSystemPrepare<Func, Params: ParamList> {
    function: Func,
    config: Params::Config,
}
pub trait ParamList {
    type Config;
    type State: Send + Sync;
    fn default_config() -> Self::Config;
}

impl<P: SystemParam> ParamList for (P,) {
    type Config = (P::Config,);
    type State = (P::State,);
    fn default_config() -> Self::Config {
        (P::Config::default(),)
    }
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
        todo!()
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
    state: Params::State,
    function: F,
}

impl<P: SystemParam, F, Out: 'static> AsSystem for FuncSystemPrepare<F, (P,)>
where
    FuncSystem<F, (P,)>: System<In = (), Out = Out>,
{
    type System = FuncSystem<F, (P,)>;
    type In = ();
    type Out = Out;

    fn as_system(self, resources: &mut Resources) -> Self::System {
        todo!()
    }
}

impl<F, P: SystemParam + 'static, Out: 'static> System for FuncSystem<F, (P,)>
where
    F: Send + Sync + 'static + FnMut(<<P as SystemParam>::State as ParamState>::Item) -> Out,
{
    type In = ();
    type Out = Out;

    fn name(&self) -> std::borrow::Cow<'static, str> {
        todo!()
    }

    fn id(&self) -> SystemId {
        todo!()
    }

    fn update(&mut self, world: &crate::World) {
        todo!()
    }

    fn archetype_component_access(&self) -> &crate::TypeAccess<crate::ArchetypeComponent> {
        todo!()
    }

    fn resource_access(&self) -> &crate::TypeAccess<std::any::TypeId> {
        todo!()
    }

    fn thread_local_execution(&self) -> crate::ThreadLocalExecution {
        todo!()
    }

    unsafe fn run_unsafe(
        &mut self,
        input: Self::In,
        world: &crate::World,
        resources: &Resources,
    ) -> Option<Self::Out> {
        todo!()
    }

    fn run_thread_local(&mut self, world: &mut crate::World, resources: &mut Resources) {
        todo!()
    }

    fn initialize(&mut self, _world: &mut crate::World, _resources: &mut Resources) {
        todo!()
    }
    //
}

fn test_system(local: &mut Local<u32>) {
    **local = 15;
}

fn test_it() {
    test_accept(test_system.system().configure(|it| it.0 = Some(32)));
}

fn test_accept<C: AsSystem<In = (), Out = ()>>(it: C) -> SystemId {
    let mut res = Resources::default();
    let sys = it.as_system(&mut res);
    sys.id()
}
