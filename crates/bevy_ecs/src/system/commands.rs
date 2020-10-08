use super::SystemId;
use crate::resource::{Resource, Resources};
use bevy_hecs::{Bundle, Component, DynamicBundle, Entity, EntityReserver, World};
use parking_lot::Mutex;
use std::{fmt, marker::PhantomData, sync::Arc};

/// A queued command to mutate the current [World] or [Resources]
pub enum Command {
    WriteWorld(Box<dyn WorldWriter>),
    WriteResources(Box<dyn ResourcesWriter>),
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Command::WriteWorld(x) => f
                .debug_tuple("WriteWorld")
                .field(&(x.as_ref() as *const dyn WorldWriter))
                .finish(),
            Command::WriteResources(x) => f
                .debug_tuple("WriteResources")
                .field(&(x.as_ref() as *const dyn ResourcesWriter))
                .finish(),
        }
    }
}

/// A [World] mutation
pub trait WorldWriter: Send + Sync {
    fn write(self: Box<Self>, world: &mut World);
}

#[derive(Debug)]
pub(crate) struct Spawn<T>
where
    T: DynamicBundle + Send + Sync + 'static,
{
    components: T,
}

impl<T> WorldWriter for Spawn<T>
where
    T: DynamicBundle + Send + Sync + 'static,
{
    fn write(self: Box<Self>, world: &mut World) {
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

impl<I> WorldWriter for SpawnBatch<I>
where
    I: IntoIterator + Send + Sync,
    I::Item: Bundle,
{
    fn write(self: Box<Self>, world: &mut World) {
        world.spawn_batch(self.components_iter);
    }
}

#[derive(Debug)]
pub(crate) struct Despawn {
    entity: Entity,
}

impl WorldWriter for Despawn {
    fn write(self: Box<Self>, world: &mut World) {
        if let Err(e) = world.despawn(self.entity) {
            log::debug!("Failed to despawn entity {:?}: {}", self.entity, e);
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

impl<T> WorldWriter for Insert<T>
where
    T: DynamicBundle + Send + Sync + 'static,
{
    fn write(self: Box<Self>, world: &mut World) {
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

impl<T> WorldWriter for InsertOne<T>
where
    T: Component,
{
    fn write(self: Box<Self>, world: &mut World) {
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

impl<T> WorldWriter for RemoveOne<T>
where
    T: Component,
{
    fn write(self: Box<Self>, world: &mut World) {
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

impl<T> WorldWriter for Remove<T>
where
    T: Bundle + Send + Sync + 'static,
{
    fn write(self: Box<Self>, world: &mut World) {
        world.remove::<T>(self.entity).unwrap();
    }
}

pub trait ResourcesWriter: Send + Sync {
    fn write(self: Box<Self>, resources: &mut Resources);
}

pub struct InsertResource<T: Resource> {
    resource: T,
}

impl<T: Resource> ResourcesWriter for InsertResource<T> {
    fn write(self: Box<Self>, resources: &mut Resources) {
        resources.insert(self.resource);
    }
}

#[derive(Debug)]
pub(crate) struct InsertLocalResource<T: Resource> {
    resource: T,
    system_id: SystemId,
}

impl<T: Resource> ResourcesWriter for InsertLocalResource<T> {
    fn write(self: Box<Self>, resources: &mut Resources) {
        resources.insert_local(self.system_id, self.resource);
    }
}

#[derive(Debug, Default)]
pub struct CommandsInternal {
    pub commands: Vec<Command>,
    pub current_entity: Option<Entity>,
    pub entity_reserver: Option<EntityReserver>,
}

impl CommandsInternal {
    pub fn spawn(&mut self, components: impl DynamicBundle + Send + Sync + 'static) -> &mut Self {
        let entity = self
            .entity_reserver
            .as_ref()
            .expect("entity reserver has not been set")
            .reserve_entity();
        self.current_entity = Some(entity);
        self.commands
            .push(Command::WriteWorld(Box::new(Insert { entity, components })));
        self
    }

    pub fn with_bundle(
        &mut self,
        components: impl DynamicBundle + Send + Sync + 'static,
    ) -> &mut Self {
        let current_entity =  self.current_entity.expect("Cannot add components because the 'current entity' is not set. You should spawn an entity first.");
        self.commands.push(Command::WriteWorld(Box::new(Insert {
            entity: current_entity,
            components,
        })));
        self
    }

    pub fn with(&mut self, component: impl Component) -> &mut Self {
        let current_entity =  self.current_entity.expect("Cannot add component because the 'current entity' is not set. You should spawn an entity first.");
        self.commands.push(Command::WriteWorld(Box::new(InsertOne {
            entity: current_entity,
            component,
        })));
        self
    }

    pub fn write_world<W: WorldWriter + 'static>(&mut self, world_writer: W) -> &mut Self {
        self.commands
            .push(Command::WriteWorld(Box::new(world_writer)));
        self
    }

    pub fn write_resources<W: ResourcesWriter + 'static>(
        &mut self,
        resources_writer: W,
    ) -> &mut Self {
        self.commands
            .push(Command::WriteResources(Box::new(resources_writer)));
        self
    }
}

/// A queue of [Command]s to run on the current [World] and [Resources]
#[derive(Debug, Default, Clone)]
pub struct Commands {
    pub commands: Arc<Mutex<CommandsInternal>>,
}

impl Commands {
    pub fn spawn(&mut self, components: impl DynamicBundle + Send + Sync + 'static) -> &mut Self {
        {
            let mut commands = self.commands.lock();
            commands.spawn(components);
        }
        self
    }

    pub fn spawn_batch<I>(&mut self, components_iter: I) -> &mut Self
    where
        I: IntoIterator + Send + Sync + 'static,
        I::Item: Bundle,
    {
        self.write_world(SpawnBatch { components_iter })
    }

    /// Despawns only the specified entity, ignoring any other consideration.
    pub fn despawn(&mut self, entity: Entity) -> &mut Self {
        self.write_world(Despawn { entity })
    }

    pub fn with(&mut self, component: impl Component) -> &mut Self {
        {
            let mut commands = self.commands.lock();
            commands.with(component);
        }
        self
    }

    pub fn with_bundle(
        &mut self,
        components: impl DynamicBundle + Send + Sync + 'static,
    ) -> &mut Self {
        {
            let mut commands = self.commands.lock();
            commands.with_bundle(components);
        }
        self
    }

    pub fn insert(
        &mut self,
        entity: Entity,
        components: impl DynamicBundle + Send + Sync + 'static,
    ) -> &mut Self {
        self.write_world(Insert { entity, components })
    }

    pub fn insert_one(&mut self, entity: Entity, component: impl Component) -> &mut Self {
        self.write_world(InsertOne { entity, component })
    }

    pub fn insert_resource<T: Resource>(&mut self, resource: T) -> &mut Self {
        self.write_resources(InsertResource { resource })
    }

    pub fn insert_local_resource<T: Resource>(
        &mut self,
        system_id: SystemId,
        resource: T,
    ) -> &mut Self {
        self.write_resources(InsertLocalResource {
            system_id,
            resource,
        })
    }

    pub fn write_world<W: WorldWriter + 'static>(&mut self, world_writer: W) -> &mut Self {
        self.commands.lock().write_world(world_writer);
        self
    }

    pub fn write_resources<W: ResourcesWriter + 'static>(
        &mut self,
        resources_writer: W,
    ) -> &mut Self {
        self.commands.lock().write_resources(resources_writer);
        self
    }

    pub fn apply(&self, world: &mut World, resources: &mut Resources) {
        let mut commands = self.commands.lock();
        for command in commands.commands.drain(..) {
            match command {
                Command::WriteWorld(writer) => {
                    writer.write(world);
                }
                Command::WriteResources(writer) => writer.write(resources),
            }
        }
    }

    pub fn current_entity(&self) -> Option<Entity> {
        let commands = self.commands.lock();
        commands.current_entity
    }

    pub fn for_current_entity(&mut self, f: impl FnOnce(Entity)) -> &mut Self {
        {
            let commands = self.commands.lock();
            let current_entity = commands
                .current_entity
                .expect("The 'current entity' is not set. You should spawn an entity first.");
            f(current_entity);
        }
        self
    }

    pub fn remove_one<T>(&mut self, entity: Entity) -> &mut Self
    where
        T: Component,
    {
        self.write_world(RemoveOne::<T> {
            entity,
            phantom: PhantomData,
        })
    }

    pub fn remove<T>(&mut self, entity: Entity) -> &mut Self
    where
        T: Bundle + Send + Sync + 'static,
    {
        self.write_world(Remove::<T> {
            entity,
            phantom: PhantomData,
        })
    }

    pub fn set_entity_reserver(&self, entity_reserver: EntityReserver) {
        self.commands.lock().entity_reserver = Some(entity_reserver);
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
            .iter()
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
            .iter()
            .map(|(a, b)| (*a, *b))
            .collect::<Vec<_>>();
        assert_eq!(results2, vec![]);
    }
}
