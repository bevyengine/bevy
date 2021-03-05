use crate::{
    bundle::Bundle,
    component::Component,
    entity::{Entities, Entity},
    world::World,
};
use bevy_utils::tracing::debug;
use std::marker::PhantomData;

/// A [World] mutation
pub trait Command: Send + Sync + 'static {
    fn write(self: Box<Self>, world: &mut World);
}

#[derive(Default)]
pub struct CommandQueue {
    commands: Vec<Box<dyn Command>>,
}

impl CommandQueue {
    pub fn apply(&mut self, world: &mut World) {
        world.flush();
        for command in self.commands.drain(..) {
            command.write(world);
        }
    }

    #[inline]
    pub fn push(&mut self, command: Box<dyn Command>) {
        self.commands.push(command);
    }
}

/// A list of commands that will be run to modify a `World`
pub struct Commands<'a> {
    queue: &'a mut CommandQueue,
    entities: &'a Entities,
    current_entity: Option<Entity>,
}

impl<'a> Commands<'a> {
    pub fn new(queue: &'a mut CommandQueue, world: &'a World) -> Self {
        Self {
            queue,
            entities: world.entities(),
            current_entity: None,
        }
    }

    /// Creates a new entity with the components contained in `bundle`.
    ///
    /// Note that `bundle` is a [Bundle], which is a collection of components. [Bundle] is automatically implemented for tuples of components. You can also create your own bundle types by deriving [`derive@Bundle`]. If you would like to spawn an entity with a single component, consider wrapping the component in a tuple (which [Bundle] is implemented for).
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
    /// # example_system.system();
    /// ```
    pub fn spawn(&mut self, bundle: impl Bundle) -> &mut Self {
        let entity = self.entities.reserve_entity();
        self.set_current_entity(entity);
        self.insert_bundle(entity, bundle);
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
    /// See [crate::world::EntityMut::insert_bundle].
    pub fn insert_bundle(&mut self, entity: Entity, bundle: impl Bundle) -> &mut Self {
        self.add_command(InsertBundle { entity, bundle })
    }

    /// Inserts a single component into `entity`.
    ///
    /// See [crate::world::EntityMut::insert].
    pub fn insert(&mut self, entity: Entity, component: impl Component) -> &mut Self {
        self.add_command(Insert { entity, component })
    }

    /// See [crate::world::EntityMut::remove].
    pub fn remove<T>(&mut self, entity: Entity) -> &mut Self
    where
        T: Component,
    {
        self.add_command(Remove::<T> {
            entity,
            phantom: PhantomData,
        })
    }

    /// See [World::insert_resource].
    pub fn insert_resource<T: Component>(&mut self, resource: T) -> &mut Self {
        self.add_command(InsertResource { resource })
    }

    /// See [crate::world::EntityMut::remove_bundle].
    pub fn remove_bundle<T>(&mut self, entity: Entity) -> &mut Self
    where
        T: Bundle,
    {
        self.add_command(RemoveBundle::<T> {
            entity,
            phantom: PhantomData,
        })
    }

    pub fn remove_resource<T: Component>(&mut self) -> &mut Self {
        self.add_command(RemoveResource::<T> {
            phantom: PhantomData,
        })
    }

    /// Adds a bundle of components to the current entity.
    ///
    /// See [`Self::with`], [`Self::current_entity`].
    pub fn with_bundle(&mut self, bundle: impl Bundle) -> &mut Self {
        let current_entity =  self.current_entity.expect("Cannot add bundle because the 'current entity' is not set. You should spawn an entity first.");
        self.queue.push(Box::new(InsertBundle {
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
    /// # example_system.system();
    /// ```
    pub fn with(&mut self, component: impl Component) -> &mut Self {
        let current_entity =  self.current_entity.expect("Cannot add component because the 'current entity' is not set. You should spawn an entity first.");
        self.queue.push(Box::new(Insert {
            entity: current_entity,
            component,
        }));
        self
    }

    /// Adds a command directly to the command list. Prefer this to [`Self::add_command_boxed`] if the type of `command` is statically known.
    pub fn add_command<C: Command>(&mut self, command: C) -> &mut Self {
        self.queue.push(Box::new(command));
        self
    }

    /// See [`Self::add_command`].
    pub fn add_command_boxed(&mut self, command: Box<dyn Command>) -> &mut Self {
        self.queue.push(command);
        self
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
}

#[derive(Debug)]
pub(crate) struct Spawn<T> {
    bundle: T,
}

impl<T> Command for Spawn<T>
where
    T: Bundle,
{
    fn write(self: Box<Self>, world: &mut World) {
        world.spawn().insert_bundle(self.bundle);
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
    I: IntoIterator + Send + Sync + 'static,
    I::Item: Bundle,
{
    fn write(self: Box<Self>, world: &mut World) {
        world.spawn_batch(self.bundles_iter);
    }
}

#[derive(Debug)]
pub(crate) struct Despawn {
    entity: Entity,
}

impl Command for Despawn {
    fn write(self: Box<Self>, world: &mut World) {
        if !world.despawn(self.entity) {
            debug!("Failed to despawn non-existent entity {:?}", self.entity);
        }
    }
}

pub struct InsertBundle<T> {
    entity: Entity,
    bundle: T,
}

impl<T> Command for InsertBundle<T>
where
    T: Bundle + 'static,
{
    fn write(self: Box<Self>, world: &mut World) {
        world.entity_mut(self.entity).insert_bundle(self.bundle);
    }
}

#[derive(Debug)]
pub(crate) struct Insert<T> {
    entity: Entity,
    component: T,
}

impl<T> Command for Insert<T>
where
    T: Component,
{
    fn write(self: Box<Self>, world: &mut World) {
        world.entity_mut(self.entity).insert(self.component);
    }
}

#[derive(Debug)]
pub(crate) struct Remove<T> {
    entity: Entity,
    phantom: PhantomData<T>,
}

impl<T> Command for Remove<T>
where
    T: Component,
{
    fn write(self: Box<Self>, world: &mut World) {
        if let Some(mut entity_mut) = world.get_entity_mut(self.entity) {
            entity_mut.remove::<T>();
        }
    }
}

#[derive(Debug)]
pub(crate) struct RemoveBundle<T> {
    entity: Entity,
    phantom: PhantomData<T>,
}

impl<T> Command for RemoveBundle<T>
where
    T: Bundle,
{
    fn write(self: Box<Self>, world: &mut World) {
        if let Some(mut entity_mut) = world.get_entity_mut(self.entity) {
            // remove intersection to gracefully handle components that were removed before running this command
            entity_mut.remove_bundle_intersection::<T>();
        }
    }
}

pub struct InsertResource<T: Component> {
    resource: T,
}

impl<T: Component> Command for InsertResource<T> {
    fn write(self: Box<Self>, world: &mut World) {
        world.insert_resource(self.resource);
    }
}

pub struct RemoveResource<T: Component> {
    phantom: PhantomData<T>,
}

impl<T: Component> Command for RemoveResource<T> {
    fn write(self: Box<Self>, world: &mut World) {
        world.remove_resource::<T>();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use crate::{
        system::{CommandQueue, Commands},
        world::World,
    };

    #[test]
    fn commands() {
        let mut world = World::default();
        let mut command_queue = CommandQueue::default();
        let entity = Commands::new(&mut command_queue, &world)
            .spawn((1u32, 2u64))
            .current_entity()
            .unwrap();
        command_queue.apply(&mut world);
        assert!(world.entities().len() == 1);
        let results = world
            .query::<(&u32, &u64)>()
            .iter(&world)
            .map(|(a, b)| (*a, *b))
            .collect::<Vec<_>>();
        assert_eq!(results, vec![(1u32, 2u64)]);
        // test entity despawn
        Commands::new(&mut command_queue, &world)
            .despawn(entity)
            .despawn(entity); // double despawn shouldn't panic
        command_queue.apply(&mut world);
        let results2 = world
            .query::<(&u32, &u64)>()
            .iter(&world)
            .map(|(a, b)| (*a, *b))
            .collect::<Vec<_>>();
        assert_eq!(results2, vec![]);
    }

    #[test]
    fn remove_components() {
        let mut world = World::default();
        let mut command_queue = CommandQueue::default();
        let entity = Commands::new(&mut command_queue, &world)
            .spawn((1u32, 2u64))
            .current_entity()
            .unwrap();
        command_queue.apply(&mut world);
        let results_before = world
            .query::<(&u32, &u64)>()
            .iter(&world)
            .map(|(a, b)| (*a, *b))
            .collect::<Vec<_>>();
        assert_eq!(results_before, vec![(1u32, 2u64)]);

        // test component removal
        Commands::new(&mut command_queue, &world)
            .remove::<u32>(entity)
            .remove_bundle::<(u32, u64)>(entity);
        command_queue.apply(&mut world);
        let results_after = world
            .query::<(&u32, &u64)>()
            .iter(&world)
            .map(|(a, b)| (*a, *b))
            .collect::<Vec<_>>();
        assert_eq!(results_after, vec![]);
        let results_after_u64 = world
            .query::<&u64>()
            .iter(&world)
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(results_after_u64, vec![]);
    }

    #[test]
    fn remove_resources() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.insert_resource(123);
            commands.insert_resource(456.0);
        }

        queue.apply(&mut world);
        assert!(world.contains_resource::<i32>());
        assert!(world.contains_resource::<f64>());

        {
            let mut commands = Commands::new(&mut queue, &world);
            // test resource removal
            commands.remove_resource::<i32>();
        }
        queue.apply(&mut world);
        assert!(!world.contains_resource::<i32>());
        assert!(world.contains_resource::<f64>());
    }
}
