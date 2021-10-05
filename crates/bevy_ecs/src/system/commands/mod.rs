mod command_queue;

use crate::{
    bundle::Bundle,
    component::Component,
    entity::{Entities, Entity},
    world::World,
};
use bevy_utils::tracing::{error, warn};
pub use command_queue::CommandQueue;
use std::marker::PhantomData;

use super::Resource;

/// A [`World`] mutation.
pub trait Command: Send + Sync + 'static {
    fn write(self, world: &mut World);
}

/// A list of commands that modify a [`World`], running at the end of the stage where they
/// have been invoked.
///
/// # Usage
///
/// `Commands` is a [`SystemParam`](crate::system::SystemParam), therefore it is declared
/// as a function parameter:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// fn my_system(mut commands: Commands) {
///    // ...
/// }
/// ```
///
/// Then, commands can be invoked by calling the methods of `commands`.
pub struct Commands<'w, 's> {
    queue: &'s mut CommandQueue,
    entities: &'w Entities,
}

impl<'w, 's> Commands<'w, 's> {
    /// Create a new `Commands` from a queue and a world.
    pub fn new(queue: &'s mut CommandQueue, world: &'w World) -> Self {
        Self {
            queue,
            entities: world.entities(),
        }
    }

    /// Creates a new empty [`Entity`] and returns an [`EntityCommands`] builder for it.
    ///
    /// To directly spawn an entity with a [`Bundle`] included, you can use
    /// [`spawn_bundle`](Self::spawn_bundle) instead of `.spawn().insert_bundle()`.
    ///
    /// See [`World::spawn`] for more details.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Label(&'static str);
    /// #[derive(Component)]
    /// struct Strength(u32);
    /// #[derive(Component)]
    /// struct Agility(u32);
    ///
    /// fn example_system(mut commands: Commands) {
    ///     // Create a new empty entity and retrieve its id.
    ///     let empty_entity = commands.spawn().id();
    ///
    ///     // Create another empty entity, then add some component to it
    ///     commands.spawn()
    ///         // adds a new component bundle to the entity
    ///         .insert_bundle((Strength(1), Agility(2)))
    ///         // adds a single component to the entity
    ///         .insert(Label("hello world"));
    /// }
    /// # example_system.system();
    /// ```
    pub fn spawn<'a>(&'a mut self) -> EntityCommands<'w, 's, 'a> {
        let entity = self.entities.reserve_entity();
        EntityCommands {
            entity,
            commands: self,
        }
    }

    /// Returns an [EntityCommands] for the given `entity` (if it exists) or spawns one if it doesn't exist.
    /// This will return [None] if the `entity` exists with a different generation.
    ///
    /// # Note
    /// Spawning a specific `entity` value is rarely the right choice. Most apps should favor [`Commands::spawn`].
    /// This method should generally only be used for sharing entities across apps, and only when they have a
    /// scheme worked out to share an ID space (which doesn't happen by default).
    pub fn get_or_spawn<'a>(&'a mut self, entity: Entity) -> EntityCommands<'w, 's, 'a> {
        self.add(GetOrSpawn { entity });
        EntityCommands {
            entity,
            commands: self,
        }
    }

    /// Spawns a [Bundle] without pre-allocating an [Entity]. The [Entity] will be allocated when
    /// this [Command] is applied.
    pub fn spawn_and_forget(&mut self, bundle: impl Bundle) {
        self.queue.push(Spawn { bundle })
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
    /// #[derive(Component)]
    /// struct Component1;
    /// #[derive(Component)]
    /// struct Component2;
    /// #[derive(Component)]
    /// struct Label(&'static str);
    /// #[derive(Component)]
    /// struct Strength(u32);
    /// #[derive(Component)]
    /// struct Agility(u32);
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
    ///         .insert_bundle((Strength(1), Agility(2)))
    ///         // or insert single components like this:
    ///         .insert(Label("hello world"));
    /// }
    /// # example_system.system();
    /// ```
    pub fn spawn_bundle<'a, T: Bundle>(&'a mut self, bundle: T) -> EntityCommands<'w, 's, 'a> {
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
    /// #[derive(Component)]
    /// struct Label(&'static str);
    /// #[derive(Component)]
    /// struct Strength(u32);
    /// #[derive(Component)]
    /// struct Agility(u32);

    /// fn example_system(mut commands: Commands) {
    ///     // Create a new, empty entity
    ///     let entity = commands.spawn().id();
    ///
    ///     commands.entity(entity)
    ///         // adds a new component bundle to the entity
    ///         .insert_bundle((Strength(1), Agility(2)))
    ///         // adds a single component to the entity
    ///         .insert(Label("hello world"));
    /// }
    /// # example_system.system();
    /// ```
    #[track_caller]
    pub fn entity<'a>(&'a mut self, entity: Entity) -> EntityCommands<'w, 's, 'a> {
        assert!(
            self.entities.contains(entity),
            "Attempting to create an EntityCommands for entity {:?}, which doesn't exist.",
            entity
        );
        EntityCommands {
            entity,
            commands: self,
        }
    }

    /// Spawns entities to the [`World`] according to the given iterator (or a type that can
    /// be converted to it).
    ///
    /// The end result of this command is equivalent to iterating `bundles_iter` and calling
    /// [`spawn`](Self::spawn) on each bundle, but it is more performant due to memory
    /// pre-allocation.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct Name(String);
    /// # #[derive(Component)]
    /// # struct Score(u32);
    /// #
    /// # fn system(mut commands: Commands) {
    /// commands.spawn_batch(vec![
    ///     (
    ///         Name("Alice".to_string()),
    ///         Score(0),
    ///     ),
    ///     (
    ///         Name("Bob".to_string()),
    ///         Score(0),
    ///     ),
    /// ]);
    /// # }
    /// # system.system();
    /// ```
    pub fn spawn_batch<I>(&mut self, bundles_iter: I)
    where
        I: IntoIterator + Send + Sync + 'static,
        I::Item: Bundle,
    {
        self.queue.push(SpawnBatch { bundles_iter });
    }

    /// For a given batch of ([Entity], [Bundle]) pairs, either spawns each [Entity] with the given
    /// bundle (if the entity does not exist), or inserts the [Bundle] (if the entity already exists).
    ///
    /// This is faster than doing equivalent operations one-by-one.
    ///
    /// # Note
    ///
    /// Spawning a specific `entity` value is rarely the right choice. Most apps should use [`Commands::spawn_batch`].
    /// This method should generally only be used for sharing entities across apps, and only when they have a scheme
    /// worked out to share an ID space (which doesn't happen by default).
    pub fn insert_or_spawn_batch<I, B>(&mut self, bundles_iter: I)
    where
        I: IntoIterator + Send + Sync + 'static,
        I::IntoIter: Iterator<Item = (Entity, B)>,
        B: Bundle,
    {
        self.queue.push(InsertOrSpawnBatch { bundles_iter });
    }

    /// Inserts a resource to the [`World`], overwriting any previous value of the same type.
    ///
    /// See [`World::insert_resource`] for more details.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # struct Scoreboard {
    /// #     current_score: u32,
    /// #     high_score: u32,
    /// # }
    /// #
    /// # fn system(mut commands: Commands) {
    /// commands.insert_resource(Scoreboard {
    ///     current_score: 0,
    ///     high_score: 0,
    /// });
    /// # }
    /// # system.system();
    /// ```
    pub fn insert_resource<T: Resource>(&mut self, resource: T) {
        self.queue.push(InsertResource { resource })
    }

    /// Removes a resource from the [`World`].
    ///
    /// See [`World::remove_resource`] for more details.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # struct Scoreboard {
    /// #     current_score: u32,
    /// #     high_score: u32,
    /// # }
    /// #
    /// # fn system(mut commands: Commands) {
    /// commands.remove_resource::<Scoreboard>();
    /// # }
    /// # system.system();
    /// ```
    pub fn remove_resource<T: Resource>(&mut self) {
        self.queue.push(RemoveResource::<T> {
            phantom: PhantomData,
        });
    }

    /// Adds a command directly to the command list.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// use bevy_ecs::system::InsertBundle;
    /// #
    /// # struct PlayerEntity { entity: Entity }
    /// # #[derive(Component)]
    /// # struct Health(u32);
    /// # #[derive(Component)]
    /// # struct Strength(u32);
    /// # #[derive(Component)]
    /// # struct Defense(u32);
    /// #
    /// # #[derive(Bundle)]
    /// # struct CombatBundle {
    /// #     health: Health,
    /// #     strength: Strength,
    /// #     defense: Defense,
    /// # }
    ///
    /// fn add_combat_stats_system(mut commands: Commands, player: Res<PlayerEntity>) {
    ///     commands.add(InsertBundle {
    ///         entity: player.entity,
    ///         bundle: CombatBundle {
    ///             health: Health(100),
    ///             strength: Strength(40),
    ///             defense: Defense(20),
    ///         },
    ///     });
    /// }
    /// # add_combat_stats_system.system();
    /// ```
    pub fn add<C: Command>(&mut self, command: C) {
        self.queue.push(command);
    }
}

/// A list of commands that will be run to modify an [entity](crate::entity).
pub struct EntityCommands<'w, 's, 'a> {
    entity: Entity,
    commands: &'a mut Commands<'w, 's>,
}

impl<'w, 's, 'a> EntityCommands<'w, 's, 'a> {
    /// Returns the [`Entity`] id of the entity.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// fn my_system(mut commands: Commands) {
    ///     let entity_id = commands.spawn().id();    
    /// }
    /// # my_system.system();
    /// ```
    #[inline]
    pub fn id(&self) -> Entity {
        self.entity
    }

    /// Adds a [`Bundle`] of components to the entity.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # struct PlayerEntity { entity: Entity }
    /// # #[derive(Component)]
    /// # struct Health(u32);
    /// # #[derive(Component)]
    /// # struct Strength(u32);
    /// # #[derive(Component)]
    /// # struct Defense(u32);
    /// #
    /// # #[derive(Bundle)]
    /// # struct CombatBundle {
    /// #     health: Health,
    /// #     strength: Strength,
    /// #     defense: Defense,
    /// # }
    /// #
    /// fn add_combat_stats_system(mut commands: Commands, player: Res<PlayerEntity>) {
    ///     commands.entity(player.entity).insert_bundle(CombatBundle {
    ///         health: Health(100),
    ///         strength: Strength(40),
    ///         defense: Defense(20),
    ///     });    
    /// }
    /// # add_combat_stats_system.system();
    /// ```
    pub fn insert_bundle(&mut self, bundle: impl Bundle) -> &mut Self {
        self.commands.add(InsertBundle {
            entity: self.entity,
            bundle,
        });
        self
    }

    /// Adds a single [`Component`] to the entity.
    ///
    /// See [`EntityMut::insert`](crate::world::EntityMut::insert) for more
    /// details.
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
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct Component1;
    /// # #[derive(Component)]
    /// # struct Component2;
    /// #
    /// fn example_system(mut commands: Commands) {
    ///     // Create a new entity with `Component1` and `Component2`
    ///     commands.spawn()
    ///         .insert(Component1)
    ///         .insert(Component2);
    ///
    ///     // The following statements are equivalent to above one.
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

    /// Removes a [`Bundle`] of components from the entity.
    ///
    /// See [`EntityMut::remove_bundle`](crate::world::EntityMut::remove_bundle) for more
    /// details.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # struct PlayerEntity { entity: Entity }
    /// #
    /// # #[derive(Component)]
    /// struct Dummy;
    /// # #[derive(Bundle)]
    /// # struct CombatBundle { a: Dummy }; // dummy field, unit bundles are not permitted.
    /// #
    /// fn remove_combat_stats_system(mut commands: Commands, player: Res<PlayerEntity>) {
    ///     commands.entity(player.entity).remove_bundle::<CombatBundle>();    
    /// }
    /// # remove_combat_stats_system.system();
    /// ```
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

    /// Removes a single component from the entity.
    ///
    /// See [`EntityMut::remove`](crate::world::EntityMut::remove) for more details.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # struct TargetEnemy { entity: Entity }
    /// # #[derive(Component)]
    /// # struct Enemy;
    /// #
    /// fn convert_enemy_system(mut commands: Commands, enemy: Res<TargetEnemy>) {
    ///     commands.entity(enemy.entity).remove::<Enemy>();
    /// }
    /// # convert_enemy_system.system();
    /// ```
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

    /// Despawns the entity.
    ///
    /// See [`World::despawn`] for more details.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # struct CharacterToRemove { entity: Entity }
    /// #
    /// fn remove_character_system(
    ///     mut commands: Commands,
    ///     character_to_remove: Res<CharacterToRemove>
    /// )
    /// {
    ///     commands.entity(character_to_remove.entity).despawn();
    /// }
    /// # remove_character_system.system();
    /// ```
    pub fn despawn(&mut self) {
        self.commands.add(Despawn {
            entity: self.entity,
        })
    }

    /// Returns the underlying [`Commands`].
    pub fn commands(&mut self) -> &mut Commands<'w, 's> {
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

pub struct GetOrSpawn {
    entity: Entity,
}

impl Command for GetOrSpawn {
    fn write(self, world: &mut World) {
        world.get_or_spawn(self.entity);
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

pub struct InsertOrSpawnBatch<I, B>
where
    I: IntoIterator + Send + Sync + 'static,
    B: Bundle,
    I::IntoIter: Iterator<Item = (Entity, B)>,
{
    pub bundles_iter: I,
}

impl<I, B> Command for InsertOrSpawnBatch<I, B>
where
    I: IntoIterator + Send + Sync + 'static,
    B: Bundle,
    I::IntoIter: Iterator<Item = (Entity, B)>,
{
    fn write(self, world: &mut World) {
        if let Err(invalid_entities) = world.insert_or_spawn_batch(self.bundles_iter) {
            error!(
                "Failed to 'insert or spawn' bundle of type {} into the following invalid entities: {:?}",
                std::any::type_name::<B>(),
                invalid_entities
            );
        }
    }
}

#[derive(Debug)]
pub struct Despawn {
    pub entity: Entity,
}

impl Command for Despawn {
    fn write(self, world: &mut World) {
        if !world.despawn(self.entity) {
            warn!("Could not despawn entity {:?} because it doesn't exist in this World.\n\
                    If this command was added to a newly spawned entity, ensure that you have not despawned that entity within the same stage.\n\
                    This may have occurred due to system order ambiguity, or if the spawning system has multiple command buffers", self.entity);
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
        if let Some(mut entity) = world.get_entity_mut(self.entity) {
            entity.insert_bundle(self.bundle);
        } else {
            panic!("Could not insert a bundle (of type `{}`) for entity {:?} because it doesn't exist in this World.\n\
                    If this command was added to a newly spawned entity, ensure that you have not despawned that entity within the same stage.\n\
                    This may have occurred due to system order ambiguity, or if the spawning system has multiple command buffers", std::any::type_name::<T>(), self.entity);
        }
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
        if let Some(mut entity) = world.get_entity_mut(self.entity) {
            entity.insert(self.component);
        } else {
            panic!("Could not add a component (of type `{}`) to entity {:?} because it doesn't exist in this World.\n\
                    If this command was added to a newly spawned entity, ensure that you have not despawned that entity within the same stage.\n\
                    This may have occurred due to system order ambiguity, or if the spawning system has multiple command buffers", std::any::type_name::<T>(), self.entity);
        }
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

pub struct InsertResource<T: Resource> {
    pub resource: T,
}

impl<T: Resource> Command for InsertResource<T> {
    fn write(self, world: &mut World) {
        world.insert_resource(self.resource);
    }
}

pub struct RemoveResource<T: Resource> {
    pub phantom: PhantomData<T>,
}

impl<T: Resource> Command for RemoveResource<T> {
    fn write(self, world: &mut World) {
        world.remove_resource::<T>();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use crate::{
        self as bevy_ecs,
        component::Component,
        system::{CommandQueue, Commands},
        world::World,
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[derive(Component)]
    #[component(storage = "SparseSet")]
    struct SparseDropCk(DropCk);

    #[derive(Component)]
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

    #[derive(Component)]
    struct W<T>(T);

    #[test]
    fn commands() {
        let mut world = World::default();
        let mut command_queue = CommandQueue::default();
        let entity = Commands::new(&mut command_queue, &world)
            .spawn_bundle((W(1u32), W(2u64)))
            .id();
        command_queue.apply(&mut world);
        assert!(world.entities().len() == 1);
        let results = world
            .query::<(&W<u32>, &W<u64>)>()
            .iter(&world)
            .map(|(a, b)| (a.0, b.0))
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
            .query::<(&W<u32>, &W<u64>)>()
            .iter(&world)
            .map(|(a, b)| (a.0, b.0))
            .collect::<Vec<_>>();
        assert_eq!(results2, vec![]);
    }

    #[test]
    fn remove_components() {
        let mut world = World::default();

        let mut command_queue = CommandQueue::default();
        let (dense_dropck, dense_is_dropped) = DropCk::new_pair();
        let (sparse_dropck, sparse_is_dropped) = DropCk::new_pair();
        let sparse_dropck = SparseDropCk(sparse_dropck);

        let entity = Commands::new(&mut command_queue, &world)
            .spawn()
            .insert_bundle((W(1u32), W(2u64), dense_dropck, sparse_dropck))
            .id();
        command_queue.apply(&mut world);
        let results_before = world
            .query::<(&W<u32>, &W<u64>)>()
            .iter(&world)
            .map(|(a, b)| (a.0, b.0))
            .collect::<Vec<_>>();
        assert_eq!(results_before, vec![(1u32, 2u64)]);

        // test component removal
        Commands::new(&mut command_queue, &world)
            .entity(entity)
            .remove::<W<u32>>()
            .remove_bundle::<(W<u32>, W<u64>, SparseDropCk, DropCk)>();

        assert_eq!(dense_is_dropped.load(Ordering::Relaxed), 0);
        assert_eq!(sparse_is_dropped.load(Ordering::Relaxed), 0);
        command_queue.apply(&mut world);
        assert_eq!(dense_is_dropped.load(Ordering::Relaxed), 1);
        assert_eq!(sparse_is_dropped.load(Ordering::Relaxed), 1);

        let results_after = world
            .query::<(&W<u32>, &W<u64>)>()
            .iter(&world)
            .map(|(a, b)| (a.0, b.0))
            .collect::<Vec<_>>();
        assert_eq!(results_after, vec![]);
        let results_after_u64 = world
            .query::<&W<u64>>()
            .iter(&world)
            .map(|v| v.0)
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
