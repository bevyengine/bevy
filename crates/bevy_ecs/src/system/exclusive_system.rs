use crate::{
    archetype::ArchetypeGeneration,
    system::{check_system_change_tick, BoxedSystem, IntoSystem, SystemId},
    world::World,
};
use std::borrow::Cow;

pub trait ExclusiveSystem: Send + Sync + 'static {
    fn name(&self) -> Cow<'static, str>;

    fn id(&self) -> SystemId;

    fn run(&mut self, world: &mut World);

    fn initialize(&mut self, world: &mut World);

    fn check_change_tick(&mut self, change_tick: u32);
}

pub struct ExclusiveSystemFn {
    func: Box<dyn FnMut(&mut World) + Send + Sync + 'static>,
    name: Cow<'static, str>,
    id: SystemId,
    last_change_tick: u32,
}

impl ExclusiveSystem for ExclusiveSystemFn {
    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn id(&self) -> SystemId {
        self.id
    }

    fn run(&mut self, world: &mut World) {
        // The previous value is saved in case this exclusive system is run by another exclusive
        // system
        let saved_last_tick = world.last_change_tick;
        world.last_change_tick = self.last_change_tick;

        (self.func)(world);

        let change_tick = world.change_tick.get_mut();
        self.last_change_tick = *change_tick;
        *change_tick += 1;

        world.last_change_tick = saved_last_tick;
    }

    fn initialize(&mut self, _: &mut World) {}

    fn check_change_tick(&mut self, change_tick: u32) {
        check_system_change_tick(&mut self.last_change_tick, change_tick, self.name.as_ref());
    }
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
            last_change_tick: 0,
        }
    }
}

pub struct ExclusiveSystemCoerced {
    system: BoxedSystem<(), ()>,
    archetype_generation: ArchetypeGeneration,
}

impl ExclusiveSystem for ExclusiveSystemCoerced {
    fn name(&self) -> Cow<'static, str> {
        self.system.name()
    }

    fn id(&self) -> SystemId {
        self.system.id()
    }

    fn run(&mut self, world: &mut World) {
        let archetypes = world.archetypes();
        let new_generation = archetypes.generation();
        let old_generation = std::mem::replace(&mut self.archetype_generation, new_generation);
        let archetype_index_range = old_generation.value()..new_generation.value();

        for archetype in archetypes.archetypes[archetype_index_range].iter() {
            self.system.new_archetype(archetype);
        }

        self.system.run((), world);
        self.system.apply_buffers(world);
    }

    fn initialize(&mut self, world: &mut World) {
        self.system.initialize(world);
    }

    fn check_change_tick(&mut self, change_tick: u32) {
        self.system.check_change_tick(change_tick);
    }
}

impl<S, Params> IntoExclusiveSystem<Params, ExclusiveSystemCoerced> for S
where
    S: IntoSystem<(), (), Params>,
{
    fn exclusive_system(self) -> ExclusiveSystemCoerced {
        ExclusiveSystemCoerced {
            system: Box::new(self.system()),
            archetype_generation: ArchetypeGeneration::initial(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        entity::Entity,
        query::With,
        schedule::{Stage, SystemStage},
        system::{Commands, IntoExclusiveSystem, Query, ResMut},
        world::World,
    };
    #[test]
    fn parallel_with_commands_as_exclusive() {
        let mut world = World::new();

        fn removal(
            mut commands: Commands,
            query: Query<Entity, With<f32>>,
            mut counter: ResMut<usize>,
        ) {
            for entity in query.iter() {
                *counter += 1;
                commands.entity(entity).remove::<f32>();
            }
        }

        let mut stage = SystemStage::parallel().with_system(removal);
        world.spawn().insert(0.0f32);
        world.insert_resource(0usize);
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(*world.get_resource::<usize>().unwrap(), 1);

        let mut stage = SystemStage::parallel().with_system(removal.exclusive_system());
        world.spawn().insert(0.0f32);
        world.insert_resource(0usize);
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(*world.get_resource::<usize>().unwrap(), 1);
    }

    #[test]
    fn update_archetype_for_exclusive_system_coerced() {
        struct Foo;

        fn spawn_entity(mut commands: crate::prelude::Commands) {
            commands.spawn().insert(Foo);
        }

        fn count_entities(query: Query<&Foo>, mut res: ResMut<Vec<usize>>) {
            res.push(query.iter().len());
        }

        let mut world = World::new();
        world.insert_resource(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_system(spawn_entity)
            .with_system(count_entities.exclusive_system());
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(*world.get_resource::<Vec<usize>>().unwrap(), vec![0, 1]);
    }
}
