mod command_queue;

use crate::{
    bundle::Bundle,
    component::Component,
    entity::{Entities, Entity},
    world::World,
};
use bevy_utils::tracing::debug;
pub use command_queue::CommandQueue;
use std::marker::PhantomData;

/// A [`World`] mutation.
pub trait Command: Send + Sync + 'static {
    fn write(self, world: &mut World);
}

/// A list of commands that will be run to modify a [`World`].
pub struct Commands<'a> {
    queue: &'a mut CommandQueue,
    entities: &'a Entities,
}

impl<'a> Commands<'a> {
    /// Create a new `Commands` from a queue and a world.
    pub fn new(queue: &'a mut CommandQueue, world: &'a World) -> Self {
        Self {
            queue,
            entities: world.entities(),
        }
    }

    /// Creates a new empty [`Entity`] and returns an [`EntityCommands`] builder for it.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// fn example_system(mut commands: Commands) {
    ///     // Create a new empty entity and retrieve its id.
    ///     let empty_entity = commands.spawn().id();
    ///
    ///     // Create another empty entity, then add some component to it
    ///     commands.spawn()
    ///         // adds a new component bundle to the entity
    ///         .insert_bundle((1usize, 2u32))
    ///         // adds a single component to the entity
    ///         .insert("hello world");
    /// }
    /// # example_system.system();
    /// ```
    pub fn spawn(&mut self) -> EntityCommands<'a, '_> {
        let entity = self.entities.reserve_entity();
        EntityCommands {
            entity,
            commands: self,
        }
    }

    /// Creates a new entity with the components contained in `bundle`.
    ///
    /// This returns an [`EntityCommands`] builder, which enables inserting more components and
    /// bundles using a "builder pattern".
    ///
    /// Note that `bundle` is a [`Bundle`], which is a collection of components. [`Bundle`] is
    /// automatically implemented for tuples of components. You can also create your own bundle
    /// types by deriving [`derive@Bundle`].
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
    ///     commands.spawn_bundle(ExampleBundle {
    ///         a: Component1,
    ///         b: Component2,
    ///     });
    ///
    ///     commands
    ///         // Create a new entity with two components using a "tuple bundle".
    ///         .spawn_bundle((Component1, Component2))
    ///         // spawn_bundle returns a builder, so you can insert more bundles like this:
    ///         .insert_bundle((1usize, 2u32))
    ///         // or insert single components like this:
    ///         .insert("hello world");
    /// }
    /// # example_system.system();
    /// ```
    pub fn spawn_bundle<'b, T: Bundle>(&'b mut self, bundle: T) -> EntityCommands<'a, 'b> {
        let mut e = self.spawn();
        e.insert_bundle(bundle);
        e
    }

    /// Returns an [`EntityCommands`] builder for the requested [`Entity`].
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// fn example_system(mut commands: Commands) {
    ///     // Create a new, empty entity
    ///     let entity = commands.spawn().id();
    ///
    ///     commands.entity(entity)
    ///         // adds a new component bundle to the entity
    ///         .insert_bundle((1usize, 2u32))
    ///         // adds a single component to the entity
    ///         .insert("hello world");
    /// }
    /// # example_system.system();
    /// ```
    pub fn entity(&mut self, entity: Entity) -> EntityCommands<'a, '_> {
        EntityCommands {
            entity,
            commands: self,
        }
    }

    /// Equivalent to iterating `bundles_iter` and calling [`Self::spawn`] on each bundle, but
    /// slightly more performant.
    pub fn spawn_batch<I>(&mut self, bundles_iter: I)
    where
        I: IntoIterator + Send + Sync + 'static,
        I::Item: Bundle,
    {
        self.queue.push(SpawnBatch { bundles_iter });
    }

    /// See [`World::insert_resource`].
    pub fn insert_resource<T: Component>(&mut self, resource: T) {
        self.queue.push(InsertResource { resource })
    }

    /// Queue a resource removal.
    pub fn remove_resource<T: Component>(&mut self) {
        self.queue.push(RemoveResource::<T> {
            phantom: PhantomData,
        });
    }

    /// Adds a command directly to the command list.
    pub fn add<C: Command>(&mut self, command: C) {
        self.queue.push(command);
    }
}

/// A list of commands that will be run to modify an [`Entity`].
pub struct EntityCommands<'a, 'b> {
    entity: Entity,
    commands: &'b mut Commands<'a>,
}

impl<'a, 'b> EntityCommands<'a, 'b> {
    /// Retrieves the current entity's unique [`Entity`] id.
    #[inline]
    pub fn id(&self) -> Entity {
        self.entity
    }

    /// Adds a [`Bundle`] of components to the current entity.
    pub fn insert_bundle(&mut self, bundle: impl Bundle) -> &mut Self {
        self.commands.add(InsertBundle {
            entity: self.entity,
            bundle,
        });
        self
    }

    /// Adds a single [`Component`] to the current entity.
    ///
    ///
    /// # Warning
    ///
    /// It's possible to call this with a bundle, but this is likely not intended and
    /// [`Self::insert_bundle`] should be used instead. If `with` is called with a bundle, the
    /// bundle itself will be added as a component instead of the bundles' inner components each
    /// being added.
    ///
    /// # Example
    ///
    /// `Self::insert` can be chained with [`Commands::spawn`].
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// struct Component1;
    /// struct Component2;
    ///
    /// fn example_system(mut commands: Commands) {
    ///     // Create a new entity with `Component1` and `Component2`
    ///     commands.spawn()
    ///         .insert(Component1)
    ///         .insert(Component2);
    ///
    ///     // Psst! These are also equivalent to the expression above!
    ///     commands.spawn().insert_bundle((Component1, Component2));
    ///     commands.spawn_bundle((Component1, Component2));
    /// }
    /// # example_system.system();
    /// ```
    pub fn insert(&mut self, component: impl Component) -> &mut Self {
        self.commands.add(Insert {
            entity: self.entity,
            component,
        });
        self
    }

    /// See [`EntityMut::remove_bundle`](crate::world::EntityMut::remove_bundle).
    pub fn remove_bundle<T>(&mut self) -> &mut Self
    where
        T: Bundle,
    {
        self.commands.add(RemoveBundle::<T> {
            entity: self.entity,
            phantom: PhantomData,
        });
        self
    }

    /// See [`EntityMut::remove`](crate::world::EntityMut::remove).
    pub fn remove<T>(&mut self) -> &mut Self
    where
        T: Component,
    {
        self.commands.add(Remove::<T> {
            entity: self.entity,
            phantom: PhantomData,
        });
        self
    }

    /// Despawns only the specified entity, not including its children.
    pub fn despawn(&mut self) {
        self.commands.add(Despawn {
            entity: self.entity,
        })
    }

    /// Returns the underlying `[Commands]`.
    pub fn commands(&mut self) -> &mut Commands<'a> {
        self.commands
    }
}

#[derive(Debug)]
pub struct Spawn<T> {
    pub bundle: T,
}

impl<T> Command for Spawn<T>
where
    T: Bundle,
{
    fn write(self, world: &mut World) {
        world.spawn().insert_bundle(self.bundle);
    }
}

pub struct SpawnBatch<I>
where
    I: IntoIterator,
    I::Item: Bundle,
{
    pub bundles_iter: I,
}

impl<I> Command for SpawnBatch<I>
where
    I: IntoIterator + Send + Sync + 'static,
    I::Item: Bundle,
{
    fn write(self, world: &mut World) {
        world.spawn_batch(self.bundles_iter);
    }
}

#[derive(Debug)]
pub struct Despawn {
    pub entity: Entity,
}

impl Command for Despawn {
    fn write(self, world: &mut World) {
        if !world.despawn(self.entity) {
            debug!("Failed to despawn non-existent entity {:?}", self.entity);
        }
    }
}

pub struct InsertBundle<T> {
    pub entity: Entity,
    pub bundle: T,
}

impl<T> Command for InsertBundle<T>
where
    T: Bundle + 'static,
{
    fn write(self, world: &mut World) {
        world.entity_mut(self.entity).insert_bundle(self.bundle);
    }
}

#[derive(Debug)]
pub struct Insert<T> {
    pub entity: Entity,
    pub component: T,
}

impl<T> Command for Insert<T>
where
    T: Component,
{
    fn write(self, world: &mut World) {
        world.entity_mut(self.entity).insert(self.component);
    }
}

#[derive(Debug)]
pub struct Remove<T> {
    pub entity: Entity,
    pub phantom: PhantomData<T>,
}

impl<T> Command for Remove<T>
where
    T: Component,
{
    fn write(self, world: &mut World) {
        if let Some(mut entity_mut) = world.get_entity_mut(self.entity) {
            entity_mut.remove::<T>();
        }
    }
}

#[derive(Debug)]
pub struct RemoveBundle<T> {
    pub entity: Entity,
    pub phantom: PhantomData<T>,
}

impl<T> Command for RemoveBundle<T>
where
    T: Bundle,
{
    fn write(self, world: &mut World) {
        if let Some(mut entity_mut) = world.get_entity_mut(self.entity) {
            // remove intersection to gracefully handle components that were removed before running
            // this command
            entity_mut.remove_bundle_intersection::<T>();
        }
    }
}

pub struct InsertResource<T: Component> {
    pub resource: T,
}

impl<T: Component> Command for InsertResource<T> {
    fn write(self, world: &mut World) {
        world.insert_resource(self.resource);
    }
}

pub struct RemoveResource<T: Component> {
    pub phantom: PhantomData<T>,
}

impl<T: Component> Command for RemoveResource<T> {
    fn write(self, world: &mut World) {
        world.remove_resource::<T>();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use crate::{
        component::{ComponentDescriptor, StorageType},
        system::{CommandQueue, Commands},
        world::World,
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[derive(Clone, Debug)]
    struct DropCk(Arc<AtomicUsize>);
    impl DropCk {
        fn new_pair() -> (Self, Arc<AtomicUsize>) {
            let atomic = Arc::new(AtomicUsize::new(0));
            (DropCk(atomic.clone()), atomic)
        }
    }

    impl Drop for DropCk {
        fn drop(&mut self) {
            self.0.as_ref().fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn commands() {
        let mut world = World::default();
        let mut command_queue = CommandQueue::default();
        let entity = Commands::new(&mut command_queue, &world)
            .spawn_bundle((1u32, 2u64))
            .id();
        command_queue.apply(&mut world);
        assert!(world.entities().len() == 1);
        let results = world
            .query::<(&u32, &u64)>()
            .iter(&world)
            .map(|(a, b)| (*a, *b))
            .collect::<Vec<_>>();
        assert_eq!(results, vec![(1u32, 2u64)]);
        // test entity despawn
        {
            let mut commands = Commands::new(&mut command_queue, &world);
            commands.entity(entity).despawn();
            commands.entity(entity).despawn(); // double despawn shouldn't panic
        }
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

        struct DenseDropCk(DropCk);
        world
            .register_component(ComponentDescriptor::new::<DropCk>(StorageType::SparseSet))
            .unwrap();

        let mut command_queue = CommandQueue::default();
        let (dense_dropck, dense_is_dropped) = DropCk::new_pair();
        let dense_dropck = DenseDropCk(dense_dropck);
        let (sparse_dropck, sparse_is_dropped) = DropCk::new_pair();

        let entity = Commands::new(&mut command_queue, &world)
            .spawn()
            .insert_bundle((1u32, 2u64, dense_dropck, sparse_dropck))
            .id();
        command_queue.apply(&mut world);
        let results_before = world
            .query::<(&u32, &u64)>()
            .iter(&world)
            .map(|(a, b)| (*a, *b))
            .collect::<Vec<_>>();
        assert_eq!(results_before, vec![(1u32, 2u64)]);

        // test component removal
        Commands::new(&mut command_queue, &world)
            .entity(entity)
            .remove::<u32>()
            .remove_bundle::<(u32, u64, DenseDropCk, DropCk)>();

        assert_eq!(dense_is_dropped.load(Ordering::Relaxed), 0);
        assert_eq!(sparse_is_dropped.load(Ordering::Relaxed), 0);
        command_queue.apply(&mut world);
        assert_eq!(dense_is_dropped.load(Ordering::Relaxed), 1);
        assert_eq!(sparse_is_dropped.load(Ordering::Relaxed), 1);

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
