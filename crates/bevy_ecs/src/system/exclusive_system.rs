pub use super::Query;
use crate::{resource::Resources, system::SystemId, BoxedSystem, IntoSystem, System, World};
use std::borrow::Cow;

pub trait ExclusiveSystem {
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

impl<S> ExclusiveSystem for S
where
    S: System<In = (), Out = ()>,
{
    fn name(&self) -> Cow<'static, str> {
        S::name(self)
    }

    fn id(&self) -> SystemId {
        S::id(self)
    }

    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        S::run(self, (), world, resources);
        S::apply_buffers(self, world, resources);
    }

    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        S::initialize(self, world, resources)
    }
}

impl ExclusiveSystem for BoxedSystem<(), ()> {
    fn name(&self) -> Cow<'static, str> {
        System::name(self.as_ref())
    }

    fn id(&self) -> SystemId {
        System::id(self.as_ref())
    }

    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        System::run(self.as_mut(), (), world, resources);
        System::apply_buffers(self.as_mut(), world, resources);
    }

    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        System::initialize(self.as_mut(), world, resources)
    }
}

impl<F> IntoSystem<(&mut World, &mut Resources), ExclusiveSystemFn> for F
where
    F: FnMut(&mut World, &mut Resources) + Send + Sync + 'static,
{
    fn system(self) -> ExclusiveSystemFn {
        ExclusiveSystemFn {
            func: Box::new(self),
            name: core::any::type_name::<F>().into(),
            id: SystemId::new(),
        }
    }
}

impl<F> IntoSystem<(&mut Resources, &mut World), ExclusiveSystemFn> for F
where
    F: FnMut(&mut Resources, &mut World) + Send + Sync + 'static,
{
    fn system(mut self) -> ExclusiveSystemFn {
        ExclusiveSystemFn {
            func: Box::new(move |world, resources| self(resources, world)),
            name: core::any::type_name::<F>().into(),
            id: SystemId::new(),
        }
    }
}

impl<F> IntoSystem<&mut World, ExclusiveSystemFn> for F
where
    F: FnMut(&mut World) + Send + Sync + 'static,
{
    fn system(mut self) -> ExclusiveSystemFn {
        ExclusiveSystemFn {
            func: Box::new(move |world, _| self(world)),
            name: core::any::type_name::<F>().into(),
            id: SystemId::new(),
        }
    }
}

impl<F> IntoSystem<&mut Resources, ExclusiveSystemFn> for F
where
    F: FnMut(&mut Resources) + Send + Sync + 'static,
{
    fn system(mut self) -> ExclusiveSystemFn {
        ExclusiveSystemFn {
            func: Box::new(move |_, resources| self(resources)),
            name: core::any::type_name::<F>().into(),
            id: SystemId::new(),
        }
    }
}
