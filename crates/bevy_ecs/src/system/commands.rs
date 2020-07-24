use super::SystemId;
use crate::resource::{Resource, Resources};
use hecs::{Bundle, Component, DynamicBundle, Entity, World};
use std::sync::{Arc, Mutex};

pub enum Command {
    WriteWorld(Box<dyn WorldWriter>),
    WriteResources(Box<dyn ResourcesWriter>),
}

pub trait WorldWriter: Send + Sync {
    fn write(self: Box<Self>, world: &mut World);
}

pub struct Spawn<T>
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

pub struct SpawnAsEntity<T>
where
    T: DynamicBundle + Send + Sync + 'static,
{
    entity: Entity,
    components: T,
}

impl<T> WorldWriter for SpawnAsEntity<T>
where
    T: DynamicBundle + Send + Sync + 'static,
{
    fn write(self: Box<Self>, world: &mut World) {
        world.spawn_as_entity(self.entity, self.components);
    }
}

pub struct SpawnBatch<I>
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

pub struct Despawn {
    entity: Entity,
}

impl WorldWriter for Despawn {
    fn write(self: Box<Self>, world: &mut World) {
        world.despawn(self.entity).unwrap();
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

pub struct InsertOne<T>
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

pub struct InsertLocalResource<T: Resource> {
    resource: T,
    system_id: SystemId,
}

impl<T: Resource> ResourcesWriter for InsertLocalResource<T> {
    fn write(self: Box<Self>, resources: &mut Resources) {
        resources.insert_local(self.system_id, self.resource);
    }
}

#[derive(Default)]
pub struct CommandsInternal {
    pub commands: Vec<Command>,
    pub current_entity: Option<Entity>,
}

impl CommandsInternal {
    pub fn spawn(&mut self, components: impl DynamicBundle + Send + Sync + 'static) -> &mut Self {
        self.spawn_as_entity(Entity::new(), components)
    }

    pub fn spawn_as_entity(
        &mut self,
        entity: Entity,
        components: impl DynamicBundle + Send + Sync + 'static,
    ) -> &mut Self {
        self.current_entity = Some(entity);
        self.commands
            .push(Command::WriteWorld(Box::new(SpawnAsEntity {
                entity,
                components,
            })));
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

#[derive(Default, Clone)]
pub struct Commands {
    pub commands: Arc<Mutex<CommandsInternal>>,
}

impl Commands {
    pub fn spawn(&mut self, components: impl DynamicBundle + Send + Sync + 'static) -> &mut Self {
        self.spawn_as_entity(Entity::new(), components)
    }

    pub fn spawn_as_entity(
        &mut self,
        entity: Entity,
        components: impl DynamicBundle + Send + Sync + 'static,
    ) -> &mut Self {
        {
            let mut commands = self.commands.lock().unwrap();
            commands.spawn_as_entity(entity, components);
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

    pub fn despawn(&mut self, entity: Entity) -> &mut Self {
        self.write_world(Despawn { entity })
    }

    pub fn with(&mut self, component: impl Component) -> &mut Self {
        {
            let mut commands = self.commands.lock().unwrap();
            commands.with(component);
        }
        self
    }

    pub fn with_bundle(
        &mut self,
        components: impl DynamicBundle + Send + Sync + 'static,
    ) -> &mut Self {
        {
            let mut commands = self.commands.lock().unwrap();
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
        self.commands.lock().unwrap().write_world(world_writer);
        self
    }

    pub fn write_resources<W: ResourcesWriter + 'static>(
        &mut self,
        resources_writer: W,
    ) -> &mut Self {
        self.commands
            .lock()
            .unwrap()
            .write_resources(resources_writer);
        self
    }

    pub fn apply(&self, world: &mut World, resources: &mut Resources) {
        let mut commands = self.commands.lock().unwrap();
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
        let commands = self.commands.lock().unwrap();
        commands.current_entity
    }

    pub fn for_current_entity(&mut self, mut func: impl FnMut(Entity)) -> &mut Self {
        {
            let commands = self.commands.lock().unwrap();
            let current_entity = commands
                .current_entity
                .expect("The 'current entity' is not set. You should spawn an entity first.");
            func(current_entity);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::Commands;
    use crate::resource::Resources;
    use hecs::World;

    #[test]
    fn command_buffer() {
        let mut world = World::default();
        let mut resources = Resources::default();
        let mut command_buffer = Commands::default();
        command_buffer.spawn((1u32, 2u64));
        command_buffer.insert_resource(3.14f32);
        command_buffer.apply(&mut world, &mut resources);
        let results = world
            .query::<(&u32, &u64)>()
            .iter()
            .map(|(a, b)| (*a, *b))
            .collect::<Vec<_>>();
        assert_eq!(results, vec![(1u32, 2u64)]);
        assert_eq!(*resources.get::<f32>().unwrap(), 3.14f32);
    }
}
