use super::SystemId;
use crate::{
    resource::{Resource, Resources},
    Bundle, Component, ComponentError, DynamicBundle, Entity, EntityReserver, World,
};
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
    bundle: T,
}

impl<T> Command for Spawn<T>
where
    T: DynamicBundle + Send + Sync + 'static,
{
    fn write(self: Box<Self>, world: &mut World, _resources: &mut Resources) {
        world.spawn(self.bundle);
    }
}

pub(crate) struct SpawnBatch<I>
where
    I: IntoIterator,
    I::Item: Bundle,
{
    bundles_iter: I,
}

impl<I> Command for SpawnBatch<I>
where
    I: IntoIterator + Send + Sync,
    I::Item: Bundle,
{
    fn write(self: Box<Self>, world: &mut World, _resources: &mut Resources) {
        world.spawn_batch(self.bundles_iter);
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
    bundle: T,
}

impl<T> Command for Insert<T>
where
    T: DynamicBundle + Send + Sync + 'static,
{
    fn write(self: Box<Self>, world: &mut World, _resources: &mut Resources) {
        world.insert(self.entity, self.bundle).unwrap();
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
            Err(ComponentError::MissingComponent(e)) => {
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

/// A list of commands that will be run to populate a `World` and `Resources`.
#[derive(Default)]
pub struct Commands {
    commands: Vec<Box<dyn Command>>,
    current_entity: Option<Entity>,
    entity_reserver: Option<EntityReserver>,
}

impl Commands {
    /// Creates a new entity with the components contained in `bundle`.
    ///
    /// Note that `bundle` is a [DynamicBundle], which is a collection of components. [DynamicBundle] is automatically implemented for tuples of components. You can also create your own bundle types by deriving [`derive@Bundle`]. If you would like to spawn an entity with a single component, consider wrapping the component in a tuple (which [DynamicBundle] is implemented for).
    ///
    /// See [`Self::set_current_entity`], [`Self::insert`].
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// struct Component1;
    /// struct Component2;
    ///
    /// #[derive(Bundle)]
    /// struct ExampleBundle {
    ///     a: Component1,
    ///     b: Component2,
    /// }
    ///
    /// fn example_system(mut commands: Commands) {
    ///     // Create a new entity with a component bundle.
    ///     commands.spawn(ExampleBundle {
    ///         a: Component1,
    ///         b: Component2,
    ///     });
    ///
    ///     // Create a new entity with a single component.
    ///     commands.spawn((Component1,));
    ///     // Create a new entity with two components.
    ///     commands.spawn((Component1, Component2));
    /// }
    /// ```
    pub fn spawn(&mut self, bundle: impl DynamicBundle + Send + Sync + 'static) -> &mut Self {
        let entity = self
            .entity_reserver
            .as_ref()
            .expect("Entity reserver has not been set.")
            .reserve_entity();
        self.set_current_entity(entity);
        self.insert(entity, bundle);
        self
    }

    /// Equivalent to iterating `bundles_iter` and calling [`Self::spawn`] on each bundle, but slightly more performant.
    pub fn spawn_batch<I>(&mut self, bundles_iter: I) -> &mut Self
    where
        I: IntoIterator + Send + Sync + 'static,
        I::Item: Bundle,
    {
        self.add_command(SpawnBatch { bundles_iter })
    }

    /// Despawns only the specified entity, not including its children.
    pub fn despawn(&mut self, entity: Entity) -> &mut Self {
        self.add_command(Despawn { entity })
    }

    /// Inserts a bundle of components into `entity`.
    ///
    /// See [`World::insert`].
    pub fn insert(
        &mut self,
        entity: Entity,
        bundle: impl DynamicBundle + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_command(Insert { entity, bundle })
    }

    /// Inserts a single component into `entity`.
    ///
    /// See [`World::insert_one`].
    pub fn insert_one(&mut self, entity: Entity, component: impl Component) -> &mut Self {
        self.add_command(InsertOne { entity, component })
    }

    pub fn insert_resource<T: Resource>(&mut self, resource: T) -> &mut Self {
        self.add_command(InsertResource { resource })
    }

    /// Insert a resource that is local to a specific system.
    ///
    /// See [`crate::System::id`].
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

    /// See [`World::remove_one`].
    pub fn remove_one<T>(&mut self, entity: Entity) -> &mut Self
    where
        T: Component,
    {
        self.add_command(RemoveOne::<T> {
            entity,
            phantom: PhantomData,
        })
    }

    /// See [`World::remove`].
    pub fn remove<T>(&mut self, entity: Entity) -> &mut Self
    where
        T: Bundle + Send + Sync + 'static,
    {
        self.add_command(Remove::<T> {
            entity,
            phantom: PhantomData,
        })
    }

    /// Adds a bundle of components to the current entity.
    ///
    /// See [`Self::with`], [`Self::current_entity`].
    pub fn with_bundle(&mut self, bundle: impl DynamicBundle + Send + Sync + 'static) -> &mut Self {
        let current_entity =  self.current_entity.expect("Cannot add bundle because the 'current entity' is not set. You should spawn an entity first.");
        self.commands.push(Box::new(Insert {
            entity: current_entity,
            bundle,
        }));
        self
    }

    /// Adds a single component to the current entity.
    ///
    /// See [`Self::with_bundle`], [`Self::current_entity`].
    ///
    /// # Warning
    ///
    /// It's possible to call this with a bundle, but this is likely not intended and [`Self::with_bundle`] should be used instead. If `with` is called with a bundle, the bundle itself will be added as a component instead of the bundles' inner components each being added.
    ///
    /// # Example
    ///
    /// `with` can be chained with [`Self::spawn`].
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// struct Component1;
    /// struct Component2;
    ///
    /// fn example_system(mut commands: Commands) {
    ///     // Create a new entity with a `Component1` and `Component2`.
    ///     commands.spawn((Component1,)).with(Component2);
    ///
    ///     // Psst! These are also equivalent to the line above!
    ///     commands.spawn((Component1, Component2));
    ///     commands.spawn(()).with(Component1).with(Component2);
    ///     #[derive(Bundle)]
    ///     struct ExampleBundle {
    ///         a: Component1,
    ///         b: Component2,
    ///     }
    ///     commands.spawn(()).with_bundle(ExampleBundle {
    ///         a: Component1,
    ///         b: Component2,
    ///     });
    /// }
    /// ```
    pub fn with(&mut self, component: impl Component) -> &mut Self {
        let current_entity =  self.current_entity.expect("Cannot add component because the 'current entity' is not set. You should spawn an entity first.");
        self.commands.push(Box::new(InsertOne {
            entity: current_entity,
            component,
        }));
        self
    }

    /// Adds a command directly to the command list. Prefer this to [`Self::add_command_boxed`] if the type of `command` is statically known.
    pub fn add_command<C: Command + 'static>(&mut self, command: C) -> &mut Self {
        self.commands.push(Box::new(command));
        self
    }

    /// See [`Self::add_command`].
    pub fn add_command_boxed(&mut self, command: Box<dyn Command>) -> &mut Self {
        self.commands.push(command);
        self
    }

    /// Runs all the stored commands on `world` and `resources`. The command buffer is emptied as a part of this call.
    pub fn apply(&mut self, world: &mut World, resources: &mut Resources) {
        for command in self.commands.drain(..) {
            command.write(world, resources);
        }
    }

    /// Returns the current entity, set by [`Self::spawn`] or with [`Self::set_current_entity`].
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
    use crate::{resource::Resources, Commands, World};

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
