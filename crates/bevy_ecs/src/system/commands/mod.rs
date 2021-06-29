mod config;
pub use config::*;

use crate::{
    bundle::Bundle,
    component::Component,
    entity::{Entities, Entity},
    world::World,
};
use std::{fmt::Debug, marker::PhantomData};

/// A [`World`] mutation.
/// If this could potentially fail, use [`FallibleCommand`].
pub trait Command: Send + Sync + 'static {
    fn write(self: Box<Self>, world: &mut World);
}

/// A [`World`] mutation that can potentially fail.
/// For an infallible variant, use [`Command`].
pub trait FallibleCommand: Send + Sync + 'static {
    type Error: Debug;

    fn try_write(self, world: &mut World) -> Result<(), Self::Error>;
}

/// A queue of [`Command`]s.
#[derive(Default)]
pub struct CommandQueue {
    commands: Vec<Box<dyn Command>>,
}

impl CommandQueue {
    /// Execute the queued [`Command`]s in the world.
    /// This clears the queue.
    pub fn apply(&mut self, world: &mut World) {
        world.flush();
        for command in self.commands.drain(..) {
            command.write(world);
        }
    }

    /// Push a boxed [`Command`] onto the queue.
    #[inline]
    pub fn push_boxed(&mut self, command: Box<dyn Command>) {
        self.commands.push(command);
    }

    /// Push a [`Command`] onto the queue.
    #[inline]
    pub fn push<T: Command>(&mut self, command: T) {
        self.push_boxed(Box::new(command));
    }
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
    pub fn spawn<'this>(&'this mut self) -> EntityCommands<'a, 'this> {
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
    pub fn spawn_bundle<'this, T: Bundle>(&'this mut self, bundle: T) -> EntityCommands<'a, 'this> {
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
    pub fn entity<'this>(&'this mut self, entity: Entity) -> EntityCommands<'a, 'this> {
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
    pub fn remove_resource<T: Component>(
        &mut self,
    ) -> FinalFallibleCommandConfig<'_, RemoveResource<T>, Self> {
        FinalFallibleCommandConfig::new(
            RemoveResource {
                phantom: PhantomData,
            },
            self,
        )
    }

    /// Adds a command directly to the command list.
    pub fn add<C: Command>(&mut self, command: C) {
        self.queue.push(command);
    }

    /// Adds a fallible command to the command list.
    pub fn add_fallible<C>(&mut self, command: C) -> FinalFallibleCommandConfig<'_, C, Self>
    where
        C: FallibleCommand,
    {
        FinalFallibleCommandConfig::new(command, self)
    }
}

impl<'a> AddCommand for Commands<'a> {
    fn add_command(&mut self, command: impl Command) {
        self.add(command);
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
    pub fn insert_bundle<T: Bundle>(
        &mut self,
        bundle: T,
    ) -> FallibleCommandConfig<'_, InsertBundle<T>, Self> {
        FallibleCommandConfig::new(
            InsertBundle {
                entity: self.entity,
                bundle,
            },
            self,
        )
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
    pub fn insert<T: Component>(
        &mut self,
        component: T,
    ) -> FallibleCommandConfig<'_, Insert<T>, Self> {
        FallibleCommandConfig::new(
            Insert {
                entity: self.entity,
                component,
            },
            self,
        )
    }

    /// See [`EntityMut::remove_bundle`](crate::world::EntityMut::remove_bundle).
    pub fn remove_bundle<T>(&mut self) -> FallibleCommandConfig<'_, RemoveBundle<T>, Self>
    where
        T: Bundle,
    {
        FallibleCommandConfig::new(
            RemoveBundle {
                entity: self.entity,
                phantom: PhantomData,
            },
            self,
        )
    }

    /// See [`EntityMut::remove`](crate::world::EntityMut::remove).
    pub fn remove<T>(&mut self) -> FallibleCommandConfig<'_, Remove<T>, Self>
    where
        T: Component,
    {
        FallibleCommandConfig::new(
            Remove {
                entity: self.entity,
                phantom: PhantomData,
            },
            self,
        )
    }

    /// Despawns only the specified entity, not including its children.
    pub fn despawn(&mut self) -> FinalFallibleCommandConfig<'_, Despawn, Self> {
        FinalFallibleCommandConfig::new(
            Despawn {
                entity: self.entity,
            },
            self,
        )
    }

    /// Returns the underlying `[Commands]`.
    pub fn commands(&mut self) -> &mut Commands<'a> {
        self.commands
    }
}

impl<'a, 'b> AddCommand for EntityCommands<'a, 'b> {
    fn add_command(&mut self, command: impl Command) {
        self.commands.add_command(command);
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
    fn write(self: Box<Self>, world: &mut World) {
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
    fn write(self: Box<Self>, world: &mut World) {
        world.spawn_batch(self.bundles_iter);
    }
}

#[derive(Debug)]
pub struct Despawn {
    pub entity: Entity,
}

/// The error resulting from [`EntityCommands::despawn`]
#[derive(Debug)]
pub struct DespawnError {
    pub entity: Entity,
}

impl FallibleCommand for Despawn {
    type Error = DespawnError;

    fn try_write(self, world: &mut World) -> Result<(), Self::Error> {
        if world.despawn(self.entity) {
            Ok(())
        } else {
            Err(DespawnError {
                entity: self.entity,
            })
        }
    }
}

pub struct InsertBundle<T> {
    pub entity: Entity,
    pub bundle: T,
}

/// The error resulting from [`EntityCommands::insert_bundle`]
/// Contains both the failed to insert bundle and the relative entity.
pub struct InsertBundleError<T> {
    pub entity: Entity,
    pub bundle: T,
}

impl<T> Debug for InsertBundleError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InsertBundleError")
            .field("entity", &self.entity)
            .field("bundle_type", &std::any::type_name::<T>())
            .finish()
    }
}

impl<T> FallibleCommand for InsertBundle<T>
where
    T: Bundle + 'static,
{
    type Error = InsertBundleError<T>;

    fn try_write(self, world: &mut World) -> Result<(), Self::Error> {
        if let Some(mut entity_mut) = world.get_entity_mut(self.entity) {
            entity_mut.insert_bundle(self.bundle);
            Ok(())
        } else {
            Err(InsertBundleError {
                entity: self.entity,
                bundle: self.bundle,
            })
        }
    }
}

#[derive(Debug)]
pub struct Insert<T> {
    pub entity: Entity,
    pub component: T,
}

/// The error resulting from [`EntityCommands::insert`]
/// Contains both the failed to insert component and the relative entity.
pub struct InsertError<T> {
    pub entity: Entity,
    pub component: T,
}

impl<T> Debug for InsertError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InsertError")
            .field("entity", &self.entity)
            .field("component_type", &std::any::type_name::<T>())
            .finish()
    }
}

impl<T> FallibleCommand for Insert<T>
where
    T: Component,
{
    type Error = InsertError<T>;

    fn try_write(self, world: &mut World) -> Result<(), Self::Error> {
        match world.get_entity_mut(self.entity) {
            Some(mut entity) => {
                entity.insert(self.component);
                Ok(())
            }
            None => Err(InsertError {
                entity: self.entity,
                component: self.component,
            }),
        }
    }
}

#[derive(Debug)]
pub struct Remove<T> {
    entity: Entity,
    phantom: PhantomData<T>,
}

/// The error resulting from [`EntityCommands::remove`]
pub struct RemoveError<T> {
    pub entity: Entity,
    phantom: PhantomData<T>,
}

impl<T> Debug for RemoveError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoveError")
            .field("entity", &self.entity)
            .field("component_type", &std::any::type_name::<T>())
            .finish()
    }
}

impl<T> FallibleCommand for Remove<T>
where
    T: Component,
{
    type Error = RemoveError<T>;

    fn try_write(self, world: &mut World) -> Result<(), Self::Error> {
        if let Some(mut entity_mut) = world.get_entity_mut(self.entity) {
            entity_mut.remove::<T>();
            Ok(())
        } else {
            Err(RemoveError {
                entity: self.entity,
                phantom: PhantomData,
            })
        }
    }
}

#[derive(Debug)]
pub struct RemoveBundle<T> {
    pub entity: Entity,
    pub phantom: PhantomData<T>,
}

/// The error resulting from [`EntityCommands::remove_bundle`]
pub struct RemoveBundleError<T> {
    pub entity: Entity,
    phantom: PhantomData<T>,
}

impl<T> Debug for RemoveBundleError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoveBundleError")
            .field("entity", &self.entity)
            .field("bundle_type", &std::any::type_name::<T>())
            .finish()
    }
}

impl<T> FallibleCommand for RemoveBundle<T>
where
    T: Bundle,
{
    type Error = RemoveBundleError<T>;

    fn try_write(self, world: &mut World) -> Result<(), Self::Error> {
        if let Some(mut entity_mut) = world.get_entity_mut(self.entity) {
            // remove intersection to gracefully handle components that were removed before running
            // this command
            entity_mut.remove_bundle_intersection::<T>();
            Ok(())
        } else {
            Err(RemoveBundleError {
                entity: self.entity,
                phantom: PhantomData,
            })
        }
    }
}

pub struct InsertResource<T: Component> {
    pub resource: T,
}

impl<T: Component> Command for InsertResource<T> {
    fn write(self: Box<Self>, world: &mut World) {
        world.insert_resource(self.resource);
    }
}

pub struct RemoveResource<T: Component> {
    pub phantom: PhantomData<T>,
}

/// The error resulting from [`Commands::remove_resource`]
pub struct RemoveResourceError<T> {
    phantom: PhantomData<T>,
}

impl<T> Debug for RemoveResourceError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoveResourceError")
            .field("resource_type", &std::any::type_name::<T>())
            .finish()
    }
}

impl<T: Component> FallibleCommand for RemoveResource<T> {
    type Error = RemoveResourceError<T>;

    fn try_write(self, world: &mut World) -> Result<(), Self::Error> {
        if world.remove_resource::<T>().is_some() {
            Ok(())
        } else {
            Err(RemoveResourceError {
                phantom: PhantomData,
            })
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use crate as bevy_ecs;
    use crate::{
        bundle::Bundle,
        component::{ComponentDescriptor, StorageType},
        entity::Entity,
        system::{CommandErrorHandler, CommandQueue, Commands, FallibleCommand},
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

    struct FailingCommand;
    impl FallibleCommand for FailingCommand {
        type Error = ();

        fn try_write(self, _: &mut World) -> Result<(), Self::Error> {
            Err(())
        }
    }

    struct SuccessfulCommand;
    impl FallibleCommand for SuccessfulCommand {
        type Error = ();

        fn try_write(self, _: &mut World) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[test]
    fn test_commands_error_handler() {
        let invoked = Arc::new(AtomicUsize::new(0));
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, &world);

            commands.insert_resource(42u32);
            let invoked_clone = invoked.clone();
            // should succeed
            commands.remove_resource::<u32>().on_err(move |_, _| {
                invoked_clone.fetch_add(1, Ordering::Relaxed);
            });

            let invoked_clone = invoked.clone();
            // should fail
            commands.remove_resource::<u32>().on_err(move |_, _| {
                invoked_clone.fetch_add(1, Ordering::Relaxed);
            });

            let invoked_clone = invoked.clone();
            // should fail
            commands.add_fallible(FailingCommand).on_err(move |_, _| {
                invoked_clone.fetch_add(1, Ordering::Relaxed);
            });

            let invoked_clone = invoked.clone();
            // should succeed
            commands
                .add_fallible(SuccessfulCommand)
                .on_err(move |_, _| {
                    invoked_clone.fetch_add(1, Ordering::Relaxed);
                });
        }
        queue.apply(&mut world);

        assert_eq!(invoked.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_entity_commands_error_handler() {
        #[derive(Bundle)]
        struct TestBundle {
            value: u32,
        }

        let invoked = Arc::new(AtomicUsize::new(0));

        let mut world = World::default();

        let valid_entity = world.spawn().id();
        let invalid_entity = Entity::new(42);

        let mut queue = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, &world);

            // EntityCommands::despawn
            let mut try_despawn = |e| {
                let invoked_clone = invoked.clone();
                commands.entity(e).despawn().on_err(move |error, _| {
                    assert_eq!(error.entity, e);
                    invoked_clone.fetch_add(1, Ordering::Relaxed);
                });
            };

            try_despawn(invalid_entity);
            try_despawn(valid_entity);

            // EntityCommands::insert
            let invoked_clone = invoked.clone();
            commands
                .entity(invalid_entity)
                .insert(42)
                .on_err(move |error, _| {
                    assert_eq!(error.entity, invalid_entity);
                    assert_eq!(error.component, 42);
                    invoked_clone.fetch_add(1, Ordering::Relaxed);
                });

            // EntityCommands::insert_bundle
            let invoked_clone = invoked.clone();
            commands
                .entity(invalid_entity)
                .insert_bundle(TestBundle { value: 42 })
                .on_err(move |error, _| {
                    assert_eq!(error.entity, invalid_entity);
                    assert_eq!(error.bundle.value, 42);
                    invoked_clone.fetch_add(1, Ordering::Relaxed);
                });

            // EntityCommands::remove
            let invoked_clone = invoked.clone();
            commands
                .entity(invalid_entity)
                .remove::<u32>()
                .on_err(move |error, _| {
                    assert_eq!(error.entity, invalid_entity);
                    invoked_clone.fetch_add(1, Ordering::Relaxed);
                });

            // EntityCommands::remove_resource
            let invoked_clone = invoked.clone();
            commands
                .entity(invalid_entity)
                .remove_bundle::<TestBundle>()
                .on_err(move |error, _| {
                    assert_eq!(error.entity, invalid_entity);
                    invoked_clone.fetch_add(1, Ordering::Relaxed);
                });
        }
        queue.apply(&mut world);

        assert_eq!(invoked.load(Ordering::Relaxed), 5);
    }

    #[test]
    #[should_panic]
    fn test_panicking_error_handler() {
        std::panic::set_hook(Box::new(|_| {})); // prevents printing of stack trace.

        let mut world = World::default();
        let mut queue = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, &world);
            let invalid_entity = Entity::new(42);
            commands
                .entity(invalid_entity)
                .despawn()
                .on_err(CommandErrorHandler::panic);
        }
        queue.apply(&mut world);
    }
}
