use super::SystemId;
use crate::resource::{Resource, Resources};
use bevy_hecs::{Bundle, Component, DynamicBundle, Entity, EntityReserver, World};
use bevy_utils::tracing::{debug, warn};
use std::marker::PhantomData;

/// A [World] mutation
pub trait Command: Send + Sync {
    fn write(self: Box<Self>, world: &mut World, resources: &mut Resources);
}

#[derive(Debug)]
pub(crate) struct Spawn<T>
where
    T: DynamicBundle + Send + Sync + 'static,
{
    components: T,
}

impl<T> Command for Spawn<T>
where
    T: DynamicBundle + Send + Sync + 'static,
{
    fn write(self: Box<Self>, world: &mut World, _resources: &mut Resources) {
        world.spawn(self.components);
    }
}

pub(crate) struct SpawnBatch<I>
where
    I: IntoIterator,
    I::Item: Bundle,
{
    components_iter: I,
}

impl<I> Command for SpawnBatch<I>
where
    I: IntoIterator + Send + Sync,
    I::Item: Bundle,
{
    fn write(self: Box<Self>, world: &mut World, _resources: &mut Resources) {
        world.spawn_batch(self.components_iter);
    }
}

#[derive(Debug)]
pub(crate) struct Despawn {
    entity: Entity,
}

impl Command for Despawn {
    fn write(self: Box<Self>, world: &mut World, _resources: &mut Resources) {
        if let Err(e) = world.despawn(self.entity) {
            debug!("Failed to despawn entity {:?}: {}", self.entity, e);
        }
    }
}

pub struct Insert<T>
where
    T: DynamicBundle + Send + Sync + 'static,
{
    entity: Entity,
    components: T,
}

impl<T> Command for Insert<T>
where
    T: DynamicBundle + Send + Sync + 'static,
{
    fn write(self: Box<Self>, world: &mut World, _resources: &mut Resources) {
        world.insert(self.entity, self.components).unwrap();
    }
}

#[derive(Debug)]
pub(crate) struct InsertOne<T>
where
    T: Component,
{
    entity: Entity,
    component: T,
}

impl<T> Command for InsertOne<T>
where
    T: Component,
{
    fn write(self: Box<Self>, world: &mut World, _resources: &mut Resources) {
        world.insert(self.entity, (self.component,)).unwrap();
    }
}

#[derive(Debug)]
pub(crate) struct RemoveOne<T>
where
    T: Component,
{
    entity: Entity,
    phantom: PhantomData<T>,
}

impl<T> Command for RemoveOne<T>
where
    T: Component,
{
    fn write(self: Box<Self>, world: &mut World, _resources: &mut Resources) {
        if world.get::<T>(self.entity).is_ok() {
            world.remove_one::<T>(self.entity).unwrap();
        }
    }
}

#[derive(Debug)]
pub(crate) struct Remove<T>
where
    T: Bundle + Send + Sync + 'static,
{
    entity: Entity,
    phantom: PhantomData<T>,
}

impl<T> Command for Remove<T>
where
    T: Bundle + Send + Sync + 'static,
{
    fn write(self: Box<Self>, world: &mut World, _resources: &mut Resources) {
        match world.remove::<T>(self.entity) {
            Ok(_) => (),
            Err(bevy_hecs::ComponentError::MissingComponent(e)) => {
                warn!(
                    "Failed to remove components {:?} with error: {}. Falling back to inefficient one-by-one component removing.",
                    std::any::type_name::<T>(),
                    e
                );
                if let Err(e) = world.remove_one_by_one::<T>(self.entity) {
                    debug!(
                        "Failed to remove components {:?} with error: {}",
                        std::any::type_name::<T>(),
                        e
                    );
                }
            }
            Err(e) => {
                debug!(
                    "Failed to remove components {:?} with error: {}",
                    std::any::type_name::<T>(),
                    e
                );
            }
        }
    }
}

pub trait ResourcesWriter: Send + Sync {
    fn write(self: Box<Self>, resources: &mut Resources);
}

pub struct InsertResource<T: Resource> {
    resource: T,
}

impl<T: Resource> Command for InsertResource<T> {
    fn write(self: Box<Self>, _world: &mut World, resources: &mut Resources) {
        resources.insert(self.resource);
    }
}

#[derive(Debug)]
pub(crate) struct InsertLocalResource<T: Resource> {
    resource: T,
    system_id: SystemId,
}

impl<T: Resource> Command for InsertLocalResource<T> {
    fn write(self: Box<Self>, _world: &mut World, resources: &mut Resources) {
        resources.insert_local(self.system_id, self.resource);
    }
}

#[derive(Default)]
pub struct Commands {
    commands: Vec<Box<dyn Command>>,
    current_entity: Option<Entity>,
    entity_reserver: Option<EntityReserver>,
}

impl Commands {
    pub fn spawn(&mut self, components: impl DynamicBundle + Send + Sync + 'static) -> &mut Self {
        let entity = self
            .entity_reserver
            .as_ref()
            .expect("entity reserver has not been set")
            .reserve_entity();
        self.current_entity = Some(entity);
        self.commands.push(Box::new(Insert { entity, components }));
        self
    }

    pub fn spawn_batch<I>(&mut self, components_iter: I) -> &mut Self
    where
        I: IntoIterator + Send + Sync + 'static,
        I::Item: Bundle,
    {
        self.add_command(SpawnBatch { components_iter })
    }

    /// Despawns only the specified entity, ignoring any other consideration.
    pub fn despawn(&mut self, entity: Entity) -> &mut Self {
        self.add_command(Despawn { entity })
    }

    pub fn insert(
        &mut self,
        entity: Entity,
        components: impl DynamicBundle + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_command(Insert { entity, components })
    }

    pub fn insert_one(&mut self, entity: Entity, component: impl Component) -> &mut Self {
        self.add_command(InsertOne { entity, component })
    }

    pub fn insert_resource<T: Resource>(&mut self, resource: T) -> &mut Self {
        self.add_command(InsertResource { resource })
    }

    pub fn insert_local_resource<T: Resource>(
        &mut self,
        system_id: SystemId,
        resource: T,
    ) -> &mut Self {
        self.add_command(InsertLocalResource {
            system_id,
            resource,
        })
    }

    pub fn remove_one<T>(&mut self, entity: Entity) -> &mut Self
    where
        T: Component,
    {
        self.add_command(RemoveOne::<T> {
            entity,
            phantom: PhantomData,
        })
    }

    pub fn remove<T>(&mut self, entity: Entity) -> &mut Self
    where
        T: Bundle + Send + Sync + 'static,
    {
        self.add_command(Remove::<T> {
            entity,
            phantom: PhantomData,
        })
    }

    pub fn with_bundle(
        &mut self,
        components: impl DynamicBundle + Send + Sync + 'static,
    ) -> &mut Self {
        let current_entity =  self.current_entity.expect("Cannot add components because the 'current entity' is not set. You should spawn an entity first.");
        self.commands.push(Box::new(Insert {
            entity: current_entity,
            components,
        }));
        self
    }

    pub fn with(&mut self, component: impl Component) -> &mut Self {
        let current_entity =  self.current_entity.expect("Cannot add component because the 'current entity' is not set. You should spawn an entity first.");
        self.commands.push(Box::new(InsertOne {
            entity: current_entity,
            component,
        }));
        self
    }

    pub fn add_command<C: Command + 'static>(&mut self, command: C) -> &mut Self {
        self.commands.push(Box::new(command));
        self
    }

    pub fn add_command_boxed(&mut self, command: Box<dyn Command>) -> &mut Self {
        self.commands.push(command);
        self
    }

    pub fn apply(&mut self, world: &mut World, resources: &mut Resources) {
        for command in self.commands.drain(..) {
            command.write(world, resources);
        }
    }

    pub fn current_entity(&self) -> Option<Entity> {
        self.current_entity
    }

    pub fn set_current_entity(&mut self, entity: Entity) {
        self.current_entity = Some(entity);
    }

    pub fn clear_current_entity(&mut self) {
        self.current_entity = None;
    }

    pub fn for_current_entity(&mut self, f: impl FnOnce(Entity)) -> &mut Self {
        let current_entity = self
            .current_entity
            .expect("The 'current entity' is not set. You should spawn an entity first.");
        f(current_entity);
        self
    }

    pub fn set_entity_reserver(&mut self, entity_reserver: EntityReserver) {
        self.entity_reserver = Some(entity_reserver);
    }
}

#[cfg(test)]
mod tests {
    use super::Commands;
    use crate::resource::Resources;
    use bevy_hecs::World;

    #[test]
    fn command_buffer() {
        let mut world = World::default();
        let mut resources = Resources::default();
        let mut command_buffer = Commands::default();
        command_buffer.set_entity_reserver(world.get_entity_reserver());
        command_buffer.spawn((1u32, 2u64));
        let entity = command_buffer.current_entity().unwrap();
        command_buffer.insert_resource(3.14f32);
        command_buffer.apply(&mut world, &mut resources);
        let results = world
            .query::<(&u32, &u64)>()
            .map(|(a, b)| (*a, *b))
            .collect::<Vec<_>>();
        assert_eq!(results, vec![(1u32, 2u64)]);
        assert_eq!(*resources.get::<f32>().unwrap(), 3.14f32);
        // test entity despawn
        command_buffer.despawn(entity);
        command_buffer.despawn(entity); // double despawn shouldn't panic
        command_buffer.apply(&mut world, &mut resources);
        let results2 = world
            .query::<(&u32, &u64)>()
            .map(|(a, b)| (*a, *b))
            .collect::<Vec<_>>();
        assert_eq!(results2, vec![]);
    }

    #[test]
    fn remove_components() {
        let mut world = World::default();
        let mut resources = Resources::default();
        let mut command_buffer = Commands::default();
        command_buffer.set_entity_reserver(world.get_entity_reserver());
        command_buffer.spawn((1u32, 2u64));
        let entity = command_buffer.current_entity().unwrap();
        command_buffer.apply(&mut world, &mut resources);
        let results_before = world
            .query::<(&u32, &u64)>()
            .map(|(a, b)| (*a, *b))
            .collect::<Vec<_>>();
        assert_eq!(results_before, vec![(1u32, 2u64)]);

        // test component removal
        command_buffer.remove_one::<u32>(entity);
        command_buffer.remove::<(u32, u64)>(entity);
        command_buffer.apply(&mut world, &mut resources);
        let results_after = world
            .query::<(&u32, &u64)>()
            .map(|(a, b)| (*a, *b))
            .collect::<Vec<_>>();
        assert_eq!(results_after, vec![]);
        let results_after_u64 = world.query::<&u64>().map(|a| *a).collect::<Vec<_>>();
        assert_eq!(results_after_u64, vec![]);
    }
}
