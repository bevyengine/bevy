use crate::{
    system::{BoxedSystem, IntoSystem, System, SystemId},
    world::World,
};
use std::borrow::Cow;

pub trait ExclusiveSystem: Send + Sync + 'static {
    fn name(&self) -> Cow<'static, str>;

    fn id(&self) -> SystemId;

    fn run(&mut self, world: &mut World);

    fn initialize(&mut self, world: &mut World);
}

pub struct ExclusiveSystemFn {
    func: Box<dyn FnMut(&mut World) + Send + Sync + 'static>,
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

    fn run(&mut self, world: &mut World) {
        (self.func)(world);
    }

    fn initialize(&mut self, _: &mut World) {}
}

pub trait IntoExclusiveSystem<Params, SystemType> {
    fn exclusive_system(self) -> SystemType;
}

impl<F> IntoExclusiveSystem<&mut World, ExclusiveSystemFn> for F
where
    F: FnMut(&mut World) + Send + Sync + 'static,
{
    fn exclusive_system(self) -> ExclusiveSystemFn {
        ExclusiveSystemFn {
            func: Box::new(self),
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

    fn run(&mut self, world: &mut World) {
        self.system.run((), world);
        self.system.apply_buffers(world);
    }

    fn initialize(&mut self, world: &mut World) {
        self.system.initialize(world);
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

#[test]
fn parallel_with_commands_as_exclusive() {
    use crate::{
        Commands, Entity, IntoExclusiveSystem, IntoSystem, ResMut, Resources, Stage, SystemStage,
        With, World,
    };
    let mut world = World::new();
    let mut resources = Resources::default();

    fn removal(
        commands: &mut Commands,
        query: Query<Entity, With<f32>>,
        mut counter: ResMut<usize>,
    ) {
        for entity in query.iter() {
            *counter += 1;
            commands.remove_one::<f32>(entity);
        }
    }

    let mut stage = SystemStage::parallel().with_system(removal.system());
    world.spawn((0.0f32,));
    resources.insert(0usize);
    stage.run(&mut world, &mut resources);
    stage.run(&mut world, &mut resources);
    assert_eq!(*resources.get::<usize>().unwrap(), 1);

    let mut stage = SystemStage::parallel().with_system(removal.exclusive_system());
    world.spawn((0.0f32,));
    resources.insert(0usize);
    stage.run(&mut world, &mut resources);
    stage.run(&mut world, &mut resources);
    assert_eq!(*resources.get::<usize>().unwrap(), 1);
}
