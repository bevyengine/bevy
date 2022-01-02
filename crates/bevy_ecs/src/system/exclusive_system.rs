use crate::{
    archetype::ArchetypeGeneration,
    system::{check_system_change_tick, BoxedSystem, IntoSystem},
    world::World,
};
use std::borrow::Cow;

pub trait ExclusiveSystem: Send + Sync + 'static {
    fn name(&self) -> Cow<'static, str>;

    fn run(&mut self, world: &mut World);

    fn initialize(&mut self, world: &mut World);

    fn check_change_tick(&mut self, change_tick: u32);
}

pub struct ExclusiveSystemFn<F> {
    func: F,
    name: Cow<'static, str>,
    last_change_tick: u32,
}

impl<F> ExclusiveSystem for ExclusiveSystemFn<F>
where
    F: FnMut(&mut World) + Send + Sync + 'static,
{
    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
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

impl<F> IntoExclusiveSystem<&mut World, ExclusiveSystemFn<F>> for F
where
    F: FnMut(&mut World) + Send + Sync + 'static,
{
    fn exclusive_system(self) -> ExclusiveSystemFn<F> {
        ExclusiveSystemFn {
            func: self,
            name: core::any::type_name::<F>().into(),
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
        self as bevy_ecs,
        component::Component,
        entity::Entity,
        query::With,
        schedule::{Stage, SystemStage},
        system::{Commands, IntoExclusiveSystem, Query, ResMut},
        world::World,
    };

    #[derive(Component)]
    struct Foo(f32);

    #[test]
    fn parallel_with_commands_as_exclusive() {
        let mut world = World::new();

        fn removal(
            mut commands: Commands,
            query: Query<Entity, With<Foo>>,
            mut counter: ResMut<usize>,
        ) {
            for entity in query.iter() {
                *counter += 1;
                commands.entity(entity).remove::<Foo>();
            }
        }

        let mut stage = SystemStage::parallel().with_system(removal);
        world.spawn().insert(Foo(0.0f32));
        world.insert_resource(0usize);
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(*world.get_resource::<usize>().unwrap(), 1);

        let mut stage = SystemStage::parallel().with_system(removal.exclusive_system());
        world.spawn().insert(Foo(0.0f32));
        world.insert_resource(0usize);
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(*world.get_resource::<usize>().unwrap(), 1);
    }

    #[test]
    fn update_archetype_for_exclusive_system_coerced() {
        fn spawn_entity(mut commands: crate::prelude::Commands) {
            commands.spawn().insert(Foo(0.0));
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
