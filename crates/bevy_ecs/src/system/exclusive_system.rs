pub use super::Query;
use crate::{resource::Resources, system::SystemId, BoxedSystem, IntoSystem, System, World};
use std::borrow::Cow;

pub trait ExclusiveSystem: Send + Sync + 'static {
    fn name(&self) -> Cow<'static, str>;

    fn id(&self) -> SystemId;

    fn run(&mut self, world: &mut World, resources: &mut Resources);

    fn initialize(&mut self, world: &mut World, resources: &mut Resources);
}

pub struct ExclusiveSystemFn {
    func: Box<dyn FnMut(&mut World, &mut Resources) + Send + Sync + 'static>,
    name: Cow<'static, str>,
    id: SystemId,
}

impl ExclusiveSystem for ExclusiveSystemFn {
    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn id(&self) -> SystemId {
        self.id
    }

    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        (self.func)(world, resources);
    }

    fn initialize(&mut self, _: &mut World, _: &mut Resources) {}
}

pub trait IntoExclusiveSystem<Params, SystemType> {
    fn exclusive_system(self) -> SystemType;
}

impl<F> IntoExclusiveSystem<(&mut World, &mut Resources), ExclusiveSystemFn> for F
where
    F: FnMut(&mut World, &mut Resources) + Send + Sync + 'static,
{
    fn exclusive_system(self) -> ExclusiveSystemFn {
        ExclusiveSystemFn {
            func: Box::new(self),
            name: core::any::type_name::<F>().into(),
            id: SystemId::new(),
        }
    }
}

impl<F> IntoExclusiveSystem<(&mut Resources, &mut World), ExclusiveSystemFn> for F
where
    F: FnMut(&mut Resources, &mut World) + Send + Sync + 'static,
{
    fn exclusive_system(mut self) -> ExclusiveSystemFn {
        ExclusiveSystemFn {
            func: Box::new(move |world, resources| self(resources, world)),
            name: core::any::type_name::<F>().into(),
            id: SystemId::new(),
        }
    }
}

impl<F> IntoExclusiveSystem<&mut World, ExclusiveSystemFn> for F
where
    F: FnMut(&mut World) + Send + Sync + 'static,
{
    fn exclusive_system(mut self) -> ExclusiveSystemFn {
        ExclusiveSystemFn {
            func: Box::new(move |world, _| self(world)),
            name: core::any::type_name::<F>().into(),
            id: SystemId::new(),
        }
    }
}

impl<F> IntoExclusiveSystem<&mut Resources, ExclusiveSystemFn> for F
where
    F: FnMut(&mut Resources) + Send + Sync + 'static,
{
    fn exclusive_system(mut self) -> ExclusiveSystemFn {
        ExclusiveSystemFn {
            func: Box::new(move |_, resources| self(resources)),
            name: core::any::type_name::<F>().into(),
            id: SystemId::new(),
        }
    }
}

pub struct ExclusiveSystemCoerced {
    system: BoxedSystem<(), ()>,
}

impl ExclusiveSystem for ExclusiveSystemCoerced {
    fn name(&self) -> Cow<'static, str> {
        self.system.name()
    }

    fn id(&self) -> SystemId {
        self.system.id()
    }

    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        self.system.run((), world, resources);
    }

    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        self.system.initialize(world, resources);
    }
}

impl<S, Params, SystemType> IntoExclusiveSystem<(Params, SystemType), ExclusiveSystemCoerced> for S
where
    S: IntoSystem<Params, SystemType>,
    SystemType: System<In = (), Out = ()>,
{
    fn exclusive_system(self) -> ExclusiveSystemCoerced {
        ExclusiveSystemCoerced {
            system: Box::new(self.system()),
        }
    }
}
