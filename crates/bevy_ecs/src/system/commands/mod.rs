mod command_queue;
mod parallel_scope;

use crate::{
    bundle::Bundle,
    entity::{Entities, Entity},
    world::{FromWorld, World},
};
use bevy_utils::tracing::{error, info};
pub use command_queue::CommandQueue;
pub use parallel_scope::*;
use std::marker::PhantomData;

use super::Resource;

/// A [`World`] mutation.
///
/// Should be used with [`Commands::add`].
///
/// # Usage
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::system::Command;
/// // Our world resource
/// #[derive(Resource, Default)]
/// struct Counter(u64);
///
/// // Our custom command
/// struct AddToCounter(u64);
///
/// impl Command for AddToCounter {
///     fn write(self, world: &mut World) {
///         let mut counter = world.get_resource_or_insert_with(Counter::default);
///         counter.0 += self.0;
///     }
/// }
///
/// fn some_system(mut commands: Commands) {
///     commands.add(AddToCounter(42));
/// }
/// ```
pub trait Command: Send + 'static {
    fn write(self, world: &mut World);
}

/// A [`Command`] queue to perform impactful changes to the [`World`].
///
/// Since each command requires exclusive access to the `World`,
/// all queued commands are automatically applied in sequence
/// only after each system in a [stage] has completed.
///
/// The command queue of a system can also be manually applied
/// by calling [`System::apply_buffers`].
///
/// Each command can be used to modify the [`World`] in arbitrary ways:
/// * spawning or despawning entities
/// * inserting components on new or existing entities
/// * inserting resources
/// * etc.
///
/// # Usage
///
/// Add `mut commands: Commands` as a function argument to your system to get a copy of this struct that will be applied at the end of the current stage.
/// Commands are almost always used as a [`SystemParam`](crate::system::SystemParam).
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// fn my_system(mut commands: Commands) {
///    // ...
/// }
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
///
/// # Implementing
///
/// Each built-in command is implemented as a separate method, e.g. [`spawn`](#method.spawn).
/// In addition to the pre-defined command methods, you can add commands with any arbitrary
/// behavior using [`Commands::add`](#method.add), which accepts any type implementing [`Command`].
///
/// Since closures and other functions implement this trait automatically, this allows one-shot,
/// anonymous custom commands.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # fn foo(mut commands: Commands) {
/// // NOTE: type inference fails here, so annotations are required on the closure.
/// commands.add(|w: &mut World| {
///     // Mutate the world however you want...
///     # todo!();
/// });
/// # }
/// ```
///
/// [stage]: crate::schedule::SystemStage
/// [`System::apply_buffers`]: crate::system::System::apply_buffers
pub struct Commands<'w, 's> {
    queue: &'s mut CommandQueue,
    entities: &'w Entities,
}

impl<'w, 's> Commands<'w, 's> {
    /// Returns a new `Commands` instance from a [`CommandQueue`] and a [`World`].
    ///
    /// It is not required to call this constructor when using `Commands` as a [system parameter].
    ///
    /// [system parameter]: crate::system::SystemParam
    pub fn new(queue: &'s mut CommandQueue, world: &'w World) -> Self {
        Self {
            queue,
            entities: world.entities(),
        }
    }

    /// Returns a new `Commands` instance from a [`CommandQueue`] and an [`Entities`] reference.
    ///
    /// It is not required to call this constructor when using `Commands` as a [system parameter].
    ///
    /// [system parameter]: crate::system::SystemParam
    pub fn new_from_entities(queue: &'s mut CommandQueue, entities: &'w Entities) -> Self {
        Self { queue, entities }
    }

    /// Pushes a [`Command`] to the queue for creating a new empty [`Entity`],
    /// and returns its corresponding [`EntityCommands`].
    ///
    /// See [`World::spawn_empty`] for more details.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
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
    ///     let empty_entity = commands.spawn_empty().id();
    ///
    ///     // Create another empty entity, then add some component to it
    ///     commands.spawn_empty()
    ///         // adds a new component bundle to the entity
    ///         .insert((Strength(1), Agility(2)))
    ///         // adds a single component to the entity
    ///         .insert(Label("hello world"));
    /// }
    /// # bevy_ecs::system::assert_is_system(example_system);
    /// ```
    ///
    /// # See also
    ///
    /// - [`spawn`](Self::spawn) to spawn an entity with a bundle.
    /// - [`spawn_batch`](Self::spawn_batch) to spawn entities with a bundle each.
    pub fn spawn_empty<'a>(&'a mut self) -> EntityCommands<'w, 's, 'a> {
        let entity = self.entities.reserve_entity();
        EntityCommands {
            entity,
            commands: self,
        }
    }

    /// Pushes a [`Command`] to the queue for creating a new [`Entity`] if the given one does not exists,
    /// and returns its corresponding [`EntityCommands`].
    ///
    /// This method silently fails by returning `EntityCommands`
    /// even if the given `Entity` cannot be spawned.
    ///
    /// See [`World::get_or_spawn`] for more details.
    ///
    /// # Note
    ///
    /// Spawning a specific `entity` value is rarely the right choice. Most apps should favor
    /// [`Commands::spawn`]. This method should generally only be used for sharing entities across
    /// apps, and only when they have a scheme worked out to share an ID space (which doesn't happen
    /// by default).
    pub fn get_or_spawn<'a>(&'a mut self, entity: Entity) -> EntityCommands<'w, 's, 'a> {
        self.add(GetOrSpawn { entity });
        EntityCommands {
            entity,
            commands: self,
        }
    }

    /// Pushes a [`Command`] to the queue for creating a new entity with the given [`Bundle`]'s components,
    /// and returns its corresponding [`EntityCommands`].
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
    ///     // Create a new entity with a single component.
    ///     commands.spawn(Component1);
    ///
    ///     // Create a new entity with a component bundle.
    ///     commands.spawn(ExampleBundle {
    ///         a: Component1,
    ///         b: Component2,
    ///     });
    ///
    ///     commands
    ///         // Create a new entity with two components using a "tuple bundle".
    ///         .spawn((Component1, Component2))
    ///         // `spawn returns a builder, so you can insert more bundles like this:
    ///         .insert((Strength(1), Agility(2)))
    ///         // or insert single components like this:
    ///         .insert(Label("hello world"));
    /// }
    /// # bevy_ecs::system::assert_is_system(example_system);
    /// ```
    ///
    /// # See also
    ///
    /// - [`spawn_empty`](Self::spawn_empty) to spawn an entity without any components.
    /// - [`spawn_batch`](Self::spawn_batch) to spawn entities with a bundle each.
    pub fn spawn<'a, T: Bundle>(&'a mut self, bundle: T) -> EntityCommands<'w, 's, 'a> {
        let mut e = self.spawn_empty();
        e.insert(bundle);
        e
    }

    /// Returns the [`EntityCommands`] for the requested [`Entity`].
    ///
    /// # Panics
    ///
    /// This method panics if the requested entity does not exist.
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
    ///     // Create a new, empty entity
    ///     let entity = commands.spawn_empty().id();
    ///
    ///     commands.entity(entity)
    ///         // adds a new component bundle to the entity
    ///         .insert((Strength(1), Agility(2)))
    ///         // adds a single component to the entity
    ///         .insert(Label("hello world"));
    /// }
    /// # bevy_ecs::system::assert_is_system(example_system);
    /// ```
    ///
    /// # See also
    ///
    /// - [`get_entity`](Self::get_entity) for the fallible version.
    #[inline]
    #[track_caller]
    pub fn entity<'a>(&'a mut self, entity: Entity) -> EntityCommands<'w, 's, 'a> {
        self.get_entity(entity).unwrap_or_else(|| {
            panic!(
                "Attempting to create an EntityCommands for entity {:?}, which doesn't exist.",
                entity
            )
        })
    }

    /// Returns the [`EntityCommands`] for the requested [`Entity`], if it exists.
    ///
    /// Returns `None` if the entity does not exist.
    ///
    /// This method does not guarantee that `EntityCommands` will be successfully applied,
    /// since another command in the queue may delete the entity before them.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Label(&'static str);

    /// fn example_system(mut commands: Commands) {
    ///     // Create a new, empty entity
    ///     let entity = commands.spawn_empty().id();
    ///
    ///     // Get the entity if it still exists, which it will in this case
    ///     if let Some(mut entity_commands) = commands.get_entity(entity) {
    ///         // adds a single component to the entity
    ///         entity_commands.insert(Label("hello world"));
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(example_system);
    /// ```
    ///
    /// # See also
    ///
    /// - [`entity`](Self::entity) for the panicking version.
    #[inline]
    #[track_caller]
    pub fn get_entity<'a>(&'a mut self, entity: Entity) -> Option<EntityCommands<'w, 's, 'a>> {
        self.entities.contains(entity).then_some(EntityCommands {
            entity,
            commands: self,
        })
    }

    /// Pushes a [`Command`] to the queue for creating entities with a particular [`Bundle`] type.
    ///
    /// `bundles_iter` is a type that can be converted into a `Bundle` iterator
    /// (it can also be a collection).
    ///
    /// This method is equivalent to iterating `bundles_iter`
    /// and calling [`spawn`](Self::spawn) on each bundle,
    /// but it is faster due to memory pre-allocation.
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
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    ///
    /// # See also
    ///
    /// - [`spawn`](Self::spawn) to spawn an entity with a bundle.
    /// - [`spawn_empty`](Self::spawn_empty) to spawn an entity without any components.
    pub fn spawn_batch<I>(&mut self, bundles_iter: I)
    where
        I: IntoIterator + Send + Sync + 'static,
        I::Item: Bundle,
    {
        self.queue.push(SpawnBatch { bundles_iter });
    }

    /// Pushes a [`Command`] to the queue for creating entities, if needed,
    /// and for adding a bundle to each entity.
    ///
    /// `bundles_iter` is a type that can be converted into an ([`Entity`], [`Bundle`]) iterator
    /// (it can also be a collection).
    ///
    /// When the command is applied,
    /// for each (`Entity`, `Bundle`) pair in the given `bundles_iter`,
    /// the `Entity` is spawned, if it does not exist already.
    /// Then, the `Bundle` is added to the entity.
    ///
    /// This method is equivalent to iterating `bundles_iter`,
    /// calling [`get_or_spawn`](Self::get_or_spawn) for each bundle,
    /// and passing it to [`insert`](EntityCommands::insert),
    /// but it is faster due to memory pre-allocation.
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

    /// Pushes a [`Command`] to the queue for inserting a [`Resource`] in the [`World`] with an inferred value.
    ///
    /// The inferred value is determined by the [`FromWorld`] trait of the resource.
    /// When the command is applied,
    /// if the resource already exists, nothing happens.
    ///
    /// See [`World::init_resource`] for more details.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Resource, Default)]
    /// # struct Scoreboard {
    /// #     current_score: u32,
    /// #     high_score: u32,
    /// # }
    /// #
    /// # fn initialise_scoreboard(mut commands: Commands) {
    /// commands.init_resource::<Scoreboard>();
    /// # }
    /// # bevy_ecs::system::assert_is_system(initialise_scoreboard);
    /// ```
    pub fn init_resource<R: Resource + FromWorld>(&mut self) {
        self.queue.push(InitResource::<R> {
            _phantom: PhantomData::<R>::default(),
        });
    }

    /// Pushes a [`Command`] to the queue for inserting a [`Resource`] in the [`World`] with a specific value.
    ///
    /// This will overwrite any previous value of the same resource type.
    ///
    /// See [`World::insert_resource`] for more details.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Resource)]
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
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn insert_resource<R: Resource>(&mut self, resource: R) {
        self.queue.push(InsertResource { resource });
    }

    /// Pushes a [`Command`] to the queue for removing a [`Resource`] from the [`World`].
    ///
    /// See [`World::remove_resource`] for more details.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Resource)]
    /// # struct Scoreboard {
    /// #     current_score: u32,
    /// #     high_score: u32,
    /// # }
    /// #
    /// # fn system(mut commands: Commands) {
    /// commands.remove_resource::<Scoreboard>();
    /// # }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn remove_resource<R: Resource>(&mut self) {
        self.queue.push(RemoveResource::<R> {
            phantom: PhantomData,
        });
    }

    /// Pushes a generic [`Command`] to the command queue.
    ///
    /// `command` can be a built-in command, custom struct that implements [`Command`] or a closure
    /// that takes [`&mut World`](World) as an argument.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::{system::Command, prelude::*};
    /// #[derive(Resource, Default)]
    /// struct Counter(u64);
    ///
    /// struct AddToCounter(u64);
    ///
    /// impl Command for AddToCounter {
    ///     fn write(self, world: &mut World) {
    ///         let mut counter = world.get_resource_or_insert_with(Counter::default);
    ///         counter.0 += self.0;
    ///     }
    /// }
    ///
    /// fn add_three_to_counter_system(mut commands: Commands) {
    ///     commands.add(AddToCounter(3));
    /// }
    /// fn add_twenty_five_to_counter_system(mut commands: Commands) {
    ///     commands.add(|world: &mut World| {
    ///         let mut counter = world.get_resource_or_insert_with(Counter::default);
    ///         counter.0 += 25;
    ///     });
    /// }

    /// # bevy_ecs::system::assert_is_system(add_three_to_counter_system);
    /// # bevy_ecs::system::assert_is_system(add_twenty_five_to_counter_system);
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
    ///     let entity_id = commands.spawn_empty().id();
    /// }
    /// # bevy_ecs::system::assert_is_system(my_system);
    /// ```
    #[inline]
    #[must_use = "Omit the .id() call if you do not need to store the `Entity` identifier."]
    pub fn id(&self) -> Entity {
        self.entity
    }

    /// Adds a [`Bundle`] of components to the entity.
    ///
    /// This will overwrite any previous value(s) of the same component type.
    ///
    /// # Panics
    ///
    /// The command will panic when applied if the associated entity does not exist.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource)]
    /// # struct PlayerEntity { entity: Entity }
    /// #[derive(Component)]
    /// struct Health(u32);
    /// #[derive(Component)]
    /// struct Strength(u32);
    /// #[derive(Component)]
    /// struct Defense(u32);
    ///
    /// #[derive(Bundle)]
    /// struct CombatBundle {
    ///     health: Health,
    ///     strength: Strength,
    /// }
    ///
    /// fn add_combat_stats_system(mut commands: Commands, player: Res<PlayerEntity>) {
    ///     commands
    ///         .entity(player.entity)
    ///         // You can insert individual components:
    ///         .insert(Defense(10))
    ///         // You can also insert pre-defined bundles of components:
    ///         .insert(CombatBundle {
    ///             health: Health(100),
    ///             strength: Strength(40),
    ///         })
    ///         // You can also insert tuples of components and bundles.
    ///         // This is equivalent to the calls above:
    ///         .insert((
    ///             Defense(10),
    ///             CombatBundle {
    ///                 health: Health(100),
    ///                 strength: Strength(40),
    ///             },
    ///         ));
    /// }
    /// # bevy_ecs::system::assert_is_system(add_combat_stats_system);
    /// ```
    pub fn insert(&mut self, bundle: impl Bundle) -> &mut Self {
        self.commands.add(Insert {
            entity: self.entity,
            bundle,
        });
        self
    }

    /// Removes a [`Bundle`] of components from the entity.
    ///
    /// See [`EntityMut::remove`](crate::world::EntityMut::remove) for more
    /// details.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Resource)]
    /// # struct PlayerEntity { entity: Entity }
    /// #[derive(Component)]
    /// struct Health(u32);
    /// #[derive(Component)]
    /// struct Strength(u32);
    /// #[derive(Component)]
    /// struct Defense(u32);
    ///
    /// #[derive(Bundle)]
    /// struct CombatBundle {
    ///     health: Health,
    ///     strength: Strength,
    /// }
    ///
    /// fn remove_combat_stats_system(mut commands: Commands, player: Res<PlayerEntity>) {
    ///     commands
    ///         .entity(player.entity)
    ///         // You can remove individual components:
    ///         .remove::<Defense>()
    ///         // You can also remove pre-defined Bundles of components:
    ///         .remove::<CombatBundle>()
    ///         // You can also remove tuples of components and bundles.
    ///         // This is equivalent to the calls above:
    ///         .remove::<(Defense, CombatBundle)>();
    /// }
    /// # bevy_ecs::system::assert_is_system(remove_combat_stats_system);
    /// ```
    pub fn remove<T>(&mut self) -> &mut Self
    where
        T: Bundle,
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
    /// # Panics
    ///
    /// The command will panic when applied if the associated entity does not exist.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Resource)]
    /// # struct CharacterToRemove { entity: Entity }
    /// #
    /// fn remove_character_system(
    ///     mut commands: Commands,
    ///     character_to_remove: Res<CharacterToRemove>
    /// )
    /// {
    ///     commands.entity(character_to_remove.entity).despawn();
    /// }
    /// # bevy_ecs::system::assert_is_system(remove_character_system);
    /// ```
    pub fn despawn(&mut self) {
        self.commands.add(Despawn {
            entity: self.entity,
        });
    }

    /// Logs the components of the entity at the info level.
    ///
    /// # Panics
    ///
    /// The command will panic when applied if the associated entity does not exist.
    pub fn log_components(&mut self) {
        self.commands.add(LogComponents {
            entity: self.entity,
        });
    }

    /// Returns the underlying [`Commands`].
    pub fn commands(&mut self) -> &mut Commands<'w, 's> {
        self.commands
    }
}

impl<F> Command for F
where
    F: FnOnce(&mut World) + Send + Sync + 'static,
{
    fn write(self, world: &mut World) {
        self(world);
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
        world.spawn(self.bundle);
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
        world.despawn(self.entity);
    }
}

pub struct Insert<T> {
    pub entity: Entity,
    pub bundle: T,
}

impl<T> Command for Insert<T>
where
    T: Bundle + 'static,
{
    fn write(self, world: &mut World) {
        if let Some(mut entity) = world.get_entity_mut(self.entity) {
            entity.insert(self.bundle);
        } else {
            panic!("error[B0003]: Could not insert a bundle (of type `{}`) for entity {:?} because it doesn't exist in this World.", std::any::type_name::<T>(), self.entity);
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
    T: Bundle,
{
    fn write(self, world: &mut World) {
        if let Some(mut entity_mut) = world.get_entity_mut(self.entity) {
            // remove intersection to gracefully handle components that were removed before running
            // this command
            entity_mut.remove_intersection::<T>();
        }
    }
}

pub struct InitResource<R: Resource + FromWorld> {
    _phantom: PhantomData<R>,
}

impl<R: Resource + FromWorld> Command for InitResource<R> {
    fn write(self, world: &mut World) {
        world.init_resource::<R>();
    }
}

pub struct InsertResource<R: Resource> {
    pub resource: R,
}

impl<R: Resource> Command for InsertResource<R> {
    fn write(self, world: &mut World) {
        world.insert_resource(self.resource);
    }
}

pub struct RemoveResource<R: Resource> {
    pub phantom: PhantomData<R>,
}

impl<R: Resource> Command for RemoveResource<R> {
    fn write(self, world: &mut World) {
        world.remove_resource::<R>();
    }
}

/// [`Command`] to log the components of a given entity. See [`EntityCommands::log_components`].
pub struct LogComponents {
    entity: Entity,
}

impl Command for LogComponents {
    fn write(self, world: &mut World) {
        let debug_infos: Vec<_> = world
            .inspect_entity(self.entity)
            .into_iter()
            .map(|component_info| component_info.name())
            .collect();
        info!("Entity {:?}: {:?}", self.entity, debug_infos);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use crate::{
        self as bevy_ecs,
        component::Component,
        system::{CommandQueue, Commands, Resource},
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

    #[derive(Component, Resource)]
    struct W<T>(T);

    fn simple_command(world: &mut World) {
        world.spawn((W(0u32), W(42u64)));
    }

    #[test]
    fn commands() {
        let mut world = World::default();
        let mut command_queue = CommandQueue::default();
        let entity = Commands::new(&mut command_queue, &world)
            .spawn((W(1u32), W(2u64)))
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

        // test adding simple (FnOnce) commands
        {
            let mut commands = Commands::new(&mut command_queue, &world);

            // set up a simple command using a closure that adds one additional entity
            commands.add(|world: &mut World| {
                world.spawn((W(42u32), W(0u64)));
            });

            // set up a simple command using a function that adds one additional entity
            commands.add(simple_command);
        }
        command_queue.apply(&mut world);
        let results3 = world
            .query::<(&W<u32>, &W<u64>)>()
            .iter(&world)
            .map(|(a, b)| (a.0, b.0))
            .collect::<Vec<_>>();

        assert_eq!(results3, vec![(42u32, 0u64), (0u32, 42u64)]);
    }

    #[test]
    fn remove_components() {
        let mut world = World::default();

        let mut command_queue = CommandQueue::default();
        let (dense_dropck, dense_is_dropped) = DropCk::new_pair();
        let (sparse_dropck, sparse_is_dropped) = DropCk::new_pair();
        let sparse_dropck = SparseDropCk(sparse_dropck);

        let entity = Commands::new(&mut command_queue, &world)
            .spawn((W(1u32), W(2u64), dense_dropck, sparse_dropck))
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
            .remove::<(W<u32>, W<u64>, SparseDropCk, DropCk)>();

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
            commands.insert_resource(W(123i32));
            commands.insert_resource(W(456.0f64));
        }

        queue.apply(&mut world);
        assert!(world.contains_resource::<W<i32>>());
        assert!(world.contains_resource::<W<f64>>());

        {
            let mut commands = Commands::new(&mut queue, &world);
            // test resource removal
            commands.remove_resource::<W<i32>>();
        }
        queue.apply(&mut world);
        assert!(!world.contains_resource::<W<i32>>());
        assert!(world.contains_resource::<W<f64>>());
    }
}
