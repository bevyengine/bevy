mod parallel_scope;

use core::{marker::PhantomData, panic::Location};

use super::{
    Deferred, IntoObserverSystem, IntoSystem, RegisterSystem, Resource, RunSystemCachedWith,
    UnregisterSystem, UnregisterSystemCached,
};
use crate::{
    self as bevy_ecs,
    bundle::{Bundle, InsertMode},
    change_detection::Mut,
    component::{Component, ComponentId, ComponentInfo, Mutable},
    entity::{Entities, Entity, EntityCloneBuilder},
    event::{Event, SendEvent},
    observer::{Observer, TriggerEvent, TriggerTargets},
    schedule::ScheduleLabel,
    system::{input::SystemInput, RunSystemWith, SystemId},
    world::{
        command_queue::RawCommandQueue, unsafe_world_cell::UnsafeWorldCell, Command, CommandQueue,
        EntityWorldMut, FromWorld, SpawnBatchIter, World,
    },
};
use bevy_ptr::OwningPtr;
use bevy_utils::tracing::{error, info};
pub use parallel_scope::*;

/// A [`Command`] queue to perform structural changes to the [`World`].
///
/// Since each command requires exclusive access to the `World`,
/// all queued commands are automatically applied in sequence
/// when the `ApplyDeferred` system runs (see [`ApplyDeferred`] documentation for more details).
///
/// Each command can be used to modify the [`World`] in arbitrary ways:
/// * spawning or despawning entities
/// * inserting components on new or existing entities
/// * inserting resources
/// * etc.
///
/// For a version of [`Commands`] that works in parallel contexts (such as
/// within [`Query::par_iter`](crate::system::Query::par_iter)) see
/// [`ParallelCommands`]
///
/// # Usage
///
/// Add `mut commands: Commands` as a function argument to your system to get a
/// copy of this struct that will be applied the next time a copy of [`ApplyDeferred`] runs.
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
/// Each built-in command is implemented as a separate method, e.g. [`Commands::spawn`].
/// In addition to the pre-defined command methods, you can add commands with any arbitrary
/// behavior using [`Commands::queue`], which accepts any type implementing [`Command`].
///
/// Since closures and other functions implement this trait automatically, this allows one-shot,
/// anonymous custom commands.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # fn foo(mut commands: Commands) {
/// // NOTE: type inference fails here, so annotations are required on the closure.
/// commands.queue(|w: &mut World| {
///     // Mutate the world however you want...
///     # todo!();
/// });
/// # }
/// ```
///
/// [`ApplyDeferred`]: crate::schedule::ApplyDeferred
pub struct Commands<'w, 's> {
    queue: InternalQueue<'s>,
    entities: &'w Entities,
}

// SAFETY: All commands [`Command`] implement [`Send`]
unsafe impl Send for Commands<'_, '_> {}

// SAFETY: `Commands` never gives access to the inner commands.
unsafe impl Sync for Commands<'_, '_> {}

const _: () = {
    type __StructFieldsAlias<'w, 's> = (Deferred<'s, CommandQueue>, &'w Entities);
    #[doc(hidden)]
    pub struct FetchState {
        state: <__StructFieldsAlias<'static, 'static> as bevy_ecs::system::SystemParam>::State,
    }
    // SAFETY: Only reads Entities
    unsafe impl bevy_ecs::system::SystemParam for Commands<'_, '_> {
        type State = FetchState;

        type Item<'w, 's> = Commands<'w, 's>;

        fn init_state(
            world: &mut World,
            system_meta: &mut bevy_ecs::system::SystemMeta,
        ) -> Self::State {
            FetchState {
                state: <__StructFieldsAlias<'_, '_> as bevy_ecs::system::SystemParam>::init_state(
                    world,
                    system_meta,
                ),
            }
        }

        unsafe fn new_archetype(
            state: &mut Self::State,
            archetype: &bevy_ecs::archetype::Archetype,
            system_meta: &mut bevy_ecs::system::SystemMeta,
        ) {
            // SAFETY: Caller guarantees the archetype is from the world used in `init_state`
            unsafe {
                <__StructFieldsAlias<'_, '_> as bevy_ecs::system::SystemParam>::new_archetype(
                    &mut state.state,
                    archetype,
                    system_meta,
                );
            };
        }

        fn apply(
            state: &mut Self::State,
            system_meta: &bevy_ecs::system::SystemMeta,
            world: &mut World,
        ) {
            <__StructFieldsAlias<'_, '_> as bevy_ecs::system::SystemParam>::apply(
                &mut state.state,
                system_meta,
                world,
            );
        }

        fn queue(
            state: &mut Self::State,
            system_meta: &bevy_ecs::system::SystemMeta,
            world: bevy_ecs::world::DeferredWorld,
        ) {
            <__StructFieldsAlias<'_, '_> as bevy_ecs::system::SystemParam>::queue(
                &mut state.state,
                system_meta,
                world,
            );
        }

        #[inline]
        unsafe fn validate_param(
            state: &Self::State,
            system_meta: &bevy_ecs::system::SystemMeta,
            world: UnsafeWorldCell,
        ) -> bool {
            <(Deferred<CommandQueue>, &Entities) as bevy_ecs::system::SystemParam>::validate_param(
                &state.state,
                system_meta,
                world,
            )
        }

        #[inline]
        unsafe fn get_param<'w, 's>(
            state: &'s mut Self::State,
            system_meta: &bevy_ecs::system::SystemMeta,
            world: UnsafeWorldCell<'w>,
            change_tick: bevy_ecs::component::Tick,
        ) -> Self::Item<'w, 's> {
            let(f0, f1) =  <(Deferred<'s, CommandQueue>, &'w Entities) as bevy_ecs::system::SystemParam>::get_param(&mut state.state, system_meta, world, change_tick);
            Commands {
                queue: InternalQueue::CommandQueue(f0),
                entities: f1,
            }
        }
    }
    // SAFETY: Only reads Entities
    unsafe impl<'w, 's> bevy_ecs::system::ReadOnlySystemParam for Commands<'w, 's>
    where
        Deferred<'s, CommandQueue>: bevy_ecs::system::ReadOnlySystemParam,
        &'w Entities: bevy_ecs::system::ReadOnlySystemParam,
    {
    }
};

enum InternalQueue<'s> {
    CommandQueue(Deferred<'s, CommandQueue>),
    RawCommandQueue(RawCommandQueue),
}

impl<'w, 's> Commands<'w, 's> {
    /// Returns a new `Commands` instance from a [`CommandQueue`] and a [`World`].
    ///
    /// It is not required to call this constructor when using `Commands` as a [system parameter].
    ///
    /// [system parameter]: crate::system::SystemParam
    pub fn new(queue: &'s mut CommandQueue, world: &'w World) -> Self {
        Self::new_from_entities(queue, &world.entities)
    }

    /// Returns a new `Commands` instance from a [`CommandQueue`] and an [`Entities`] reference.
    ///
    /// It is not required to call this constructor when using `Commands` as a [system parameter].
    ///
    /// [system parameter]: crate::system::SystemParam
    pub fn new_from_entities(queue: &'s mut CommandQueue, entities: &'w Entities) -> Self {
        Self {
            queue: InternalQueue::CommandQueue(Deferred(queue)),
            entities,
        }
    }

    /// Returns a new `Commands` instance from a [`RawCommandQueue`] and an [`Entities`] reference.
    ///
    /// This is used when constructing [`Commands`] from a [`DeferredWorld`](crate::world::DeferredWorld).
    ///
    /// # Safety
    ///
    /// * Caller ensures that `queue` must outlive 'w
    pub(crate) unsafe fn new_raw_from_entities(
        queue: RawCommandQueue,
        entities: &'w Entities,
    ) -> Self {
        Self {
            queue: InternalQueue::RawCommandQueue(queue),
            entities,
        }
    }

    /// Returns a [`Commands`] with a smaller lifetime.
    /// This is useful if you have `&mut Commands` but need `Commands`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// fn my_system(mut commands: Commands) {
    ///     // We do our initialization in a separate function,
    ///     // which expects an owned `Commands`.
    ///     do_initialization(commands.reborrow());
    ///
    ///     // Since we only reborrowed the commands instead of moving them, we can still use them.
    ///     commands.spawn_empty();
    /// }
    /// #
    /// # fn do_initialization(_: Commands) {}
    /// ```
    pub fn reborrow(&mut self) -> Commands<'w, '_> {
        Commands {
            queue: match &mut self.queue {
                InternalQueue::CommandQueue(queue) => InternalQueue::CommandQueue(queue.reborrow()),
                InternalQueue::RawCommandQueue(queue) => {
                    InternalQueue::RawCommandQueue(queue.clone())
                }
            },
            entities: self.entities,
        }
    }

    /// Take all commands from `other` and append them to `self`, leaving `other` empty
    pub fn append(&mut self, other: &mut CommandQueue) {
        match &mut self.queue {
            InternalQueue::CommandQueue(queue) => queue.bytes.append(&mut other.bytes),
            InternalQueue::RawCommandQueue(queue) => {
                // SAFETY: Pointers in `RawCommandQueue` are never null
                unsafe { queue.bytes.as_mut() }.append(&mut other.bytes);
            }
        }
    }

    /// Reserves a new empty [`Entity`] to be spawned, and returns its corresponding [`EntityCommands`].
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
    pub fn spawn_empty(&mut self) -> EntityCommands {
        let entity = self.entities.reserve_entity();
        EntityCommands {
            entity,
            commands: self.reborrow(),
        }
    }

    /// Pushes a [`Command`] to the queue for creating a new [`Entity`] if the given one does not exists,
    /// and returns its corresponding [`EntityCommands`].
    ///
    /// This method silently fails by returning [`EntityCommands`]
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
    #[deprecated(since = "0.15.0", note = "use Commands::spawn instead")]
    pub fn get_or_spawn(&mut self, entity: Entity) -> EntityCommands {
        self.queue(move |world: &mut World| {
            #[allow(deprecated)]
            world.get_or_spawn(entity);
        });
        EntityCommands {
            entity,
            commands: self.reborrow(),
        }
    }

    /// Pushes a [`Command`] to the queue for creating a new entity with the given [`Bundle`]'s components,
    /// and returns its corresponding [`EntityCommands`].
    ///
    /// In case multiple bundles of the same [`Bundle`] type need to be spawned,
    /// [`spawn_batch`](Self::spawn_batch) should be used for better performance.
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
    #[track_caller]
    pub fn spawn<T: Bundle>(&mut self, bundle: T) -> EntityCommands {
        let mut entity = self.spawn_empty();
        entity.insert(bundle);
        entity
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
    pub fn entity(&mut self, entity: Entity) -> EntityCommands {
        #[inline(never)]
        #[cold]
        #[track_caller]
        fn panic_no_entity(entity: Entity) -> ! {
            panic!(
                "Attempting to create an EntityCommands for entity {entity:?}, which doesn't exist.",
            );
        }

        match self.get_entity(entity) {
            Some(entity) => entity,
            None => panic_no_entity(entity),
        }
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
    pub fn get_entity(&mut self, entity: Entity) -> Option<EntityCommands> {
        self.entities.contains(entity).then_some(EntityCommands {
            entity,
            commands: self.reborrow(),
        })
    }

    /// Pushes a [`Command`] to the queue for creating entities with a particular [`Bundle`] type.
    ///
    /// `bundles_iter` is a type that can be converted into a [`Bundle`] iterator
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
    #[track_caller]
    pub fn spawn_batch<I>(&mut self, bundles_iter: I)
    where
        I: IntoIterator + Send + Sync + 'static,
        I::Item: Bundle,
    {
        self.queue(spawn_batch(bundles_iter));
    }

    /// Pushes a generic [`Command`] to the command queue.
    ///
    /// `command` can be a built-in command, custom struct that implements [`Command`] or a closure
    /// that takes [`&mut World`](World) as an argument.
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::{world::Command, prelude::*};
    /// #[derive(Resource, Default)]
    /// struct Counter(u64);
    ///
    /// struct AddToCounter(u64);
    ///
    /// impl Command for AddToCounter {
    ///     fn apply(self, world: &mut World) {
    ///         let mut counter = world.get_resource_or_insert_with(Counter::default);
    ///         counter.0 += self.0;
    ///     }
    /// }
    ///
    /// fn add_three_to_counter_system(mut commands: Commands) {
    ///     commands.queue(AddToCounter(3));
    /// }
    /// fn add_twenty_five_to_counter_system(mut commands: Commands) {
    ///     commands.queue(|world: &mut World| {
    ///         let mut counter = world.get_resource_or_insert_with(Counter::default);
    ///         counter.0 += 25;
    ///     });
    /// }
    /// # bevy_ecs::system::assert_is_system(add_three_to_counter_system);
    /// # bevy_ecs::system::assert_is_system(add_twenty_five_to_counter_system);
    /// ```
    pub fn queue<C: Command>(&mut self, command: C) {
        match &mut self.queue {
            InternalQueue::CommandQueue(queue) => {
                queue.push(command);
            }
            InternalQueue::RawCommandQueue(queue) => {
                // SAFETY: `RawCommandQueue` is only every constructed in `Commands::new_raw_from_entities`
                // where the caller of that has ensured that `queue` outlives `self`
                unsafe {
                    queue.push(command);
                }
            }
        }
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
    #[track_caller]
    pub fn insert_or_spawn_batch<I, B>(&mut self, bundles_iter: I)
    where
        I: IntoIterator<Item = (Entity, B)> + Send + Sync + 'static,
        B: Bundle,
    {
        self.queue(insert_or_spawn_batch(bundles_iter));
    }

    /// Pushes a [`Command`] to the queue for adding a [`Bundle`] type to a batch of [`Entities`](Entity).
    ///
    /// A batch can be any type that implements [`IntoIterator`] containing `(Entity, Bundle)` tuples,
    /// such as a [`Vec<(Entity, Bundle)>`] or an array `[(Entity, Bundle); N]`.
    ///
    /// When the command is applied, for each `(Entity, Bundle)` pair in the given batch,
    /// the `Bundle` is added to the `Entity`, overwriting any existing components shared by the `Bundle`.
    ///
    /// This method is equivalent to iterating the batch,
    /// calling [`entity`](Self::entity) for each pair,
    /// and passing the bundle to [`insert`](EntityCommands::insert),
    /// but it is faster due to memory pre-allocation.
    ///
    /// # Panics
    ///
    /// This command panics if any of the given entities do not exist.
    ///
    /// For the non-panicking version, see [`try_insert_batch`](Self::try_insert_batch).
    #[track_caller]
    pub fn insert_batch<I, B>(&mut self, batch: I)
    where
        I: IntoIterator<Item = (Entity, B)> + Send + Sync + 'static,
        B: Bundle,
    {
        self.queue(insert_batch(batch));
    }

    /// Pushes a [`Command`] to the queue for adding a [`Bundle`] type to a batch of [`Entities`](Entity).
    ///
    /// A batch can be any type that implements [`IntoIterator`] containing `(Entity, Bundle)` tuples,
    /// such as a [`Vec<(Entity, Bundle)>`] or an array `[(Entity, Bundle); N]`.
    ///
    /// When the command is applied, for each `(Entity, Bundle)` pair in the given batch,
    /// the `Bundle` is added to the `Entity`, except for any components already present on the `Entity`.
    ///
    /// This method is equivalent to iterating the batch,
    /// calling [`entity`](Self::entity) for each pair,
    /// and passing the bundle to [`insert_if_new`](EntityCommands::insert_if_new),
    /// but it is faster due to memory pre-allocation.
    ///
    /// # Panics
    ///
    /// This command panics if any of the given entities do not exist.
    ///
    /// For the non-panicking version, see [`try_insert_batch_if_new`](Self::try_insert_batch_if_new).
    #[track_caller]
    pub fn insert_batch_if_new<I, B>(&mut self, batch: I)
    where
        I: IntoIterator<Item = (Entity, B)> + Send + Sync + 'static,
        B: Bundle,
    {
        self.queue(insert_batch_if_new(batch));
    }

    /// Pushes a [`Command`] to the queue for adding a [`Bundle`] type to a batch of [`Entities`](Entity).
    ///
    /// A batch can be any type that implements [`IntoIterator`] containing `(Entity, Bundle)` tuples,
    /// such as a [`Vec<(Entity, Bundle)>`] or an array `[(Entity, Bundle); N]`.
    ///
    /// When the command is applied, for each `(Entity, Bundle)` pair in the given batch,
    /// the `Bundle` is added to the `Entity`, overwriting any existing components shared by the `Bundle`.
    ///
    /// This method is equivalent to iterating the batch,
    /// calling [`get_entity`](Self::get_entity) for each pair,
    /// and passing the bundle to [`insert`](EntityCommands::insert),
    /// but it is faster due to memory pre-allocation.
    ///
    /// This command silently fails by ignoring any entities that do not exist.
    ///
    /// For the panicking version, see [`insert_batch`](Self::insert_batch).
    #[track_caller]
    pub fn try_insert_batch<I, B>(&mut self, batch: I)
    where
        I: IntoIterator<Item = (Entity, B)> + Send + Sync + 'static,
        B: Bundle,
    {
        self.queue(try_insert_batch(batch));
    }

    /// Pushes a [`Command`] to the queue for adding a [`Bundle`] type to a batch of [`Entities`](Entity).
    ///
    /// A batch can be any type that implements [`IntoIterator`] containing `(Entity, Bundle)` tuples,
    /// such as a [`Vec<(Entity, Bundle)>`] or an array `[(Entity, Bundle); N]`.
    ///
    /// When the command is applied, for each `(Entity, Bundle)` pair in the given batch,
    /// the `Bundle` is added to the `Entity`, except for any components already present on the `Entity`.
    ///
    /// This method is equivalent to iterating the batch,
    /// calling [`get_entity`](Self::get_entity) for each pair,
    /// and passing the bundle to [`insert_if_new`](EntityCommands::insert_if_new),
    /// but it is faster due to memory pre-allocation.
    ///
    /// This command silently fails by ignoring any entities that do not exist.
    ///
    /// For the panicking version, see [`insert_batch_if_new`](Self::insert_batch_if_new).
    #[track_caller]
    pub fn try_insert_batch_if_new<I, B>(&mut self, batch: I)
    where
        I: IntoIterator<Item = (Entity, B)> + Send + Sync + 'static,
        B: Bundle,
    {
        self.queue(try_insert_batch_if_new(batch));
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
    /// # fn initialize_scoreboard(mut commands: Commands) {
    /// commands.init_resource::<Scoreboard>();
    /// # }
    /// # bevy_ecs::system::assert_is_system(initialize_scoreboard);
    /// ```
    #[track_caller]
    pub fn init_resource<R: Resource + FromWorld>(&mut self) {
        self.queue(init_resource::<R>);
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
    #[track_caller]
    pub fn insert_resource<R: Resource>(&mut self, resource: R) {
        self.queue(insert_resource(resource));
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
        self.queue(remove_resource::<R>);
    }

    /// Runs the system corresponding to the given [`SystemId`].
    /// Systems are ran in an exclusive and single threaded way.
    /// Running slow systems can become a bottleneck.
    ///
    /// Calls [`World::run_system`](World::run_system).
    ///
    /// There is no way to get the output of a system when run as a command, because the
    /// execution of the system happens later. To get the output of a system, use
    /// [`World::run_system`] or [`World::run_system_with`] instead of running the system as a command.
    pub fn run_system(&mut self, id: SystemId) {
        self.run_system_with(id, ());
    }

    /// Runs the system corresponding to the given [`SystemId`].
    /// Systems are ran in an exclusive and single threaded way.
    /// Running slow systems can become a bottleneck.
    ///
    /// Calls [`World::run_system_with`](World::run_system_with).
    ///
    /// There is no way to get the output of a system when run as a command, because the
    /// execution of the system happens later. To get the output of a system, use
    /// [`World::run_system`] or [`World::run_system_with`] instead of running the system as a command.
    pub fn run_system_with<I>(&mut self, id: SystemId<I>, input: I::Inner<'static>)
    where
        I: SystemInput<Inner<'static>: Send> + 'static,
    {
        self.queue(RunSystemWith::new_with_input(id, input));
    }

    /// Registers a system and returns a [`SystemId`] so it can later be called by [`World::run_system`].
    ///
    /// It's possible to register the same systems more than once, they'll be stored separately.
    ///
    /// This is different from adding systems to a [`Schedule`](crate::schedule::Schedule),
    /// because the [`SystemId`] that is returned can be used anywhere in the [`World`] to run the associated system.
    /// This allows for running systems in a push-based fashion.
    /// Using a [`Schedule`](crate::schedule::Schedule) is still preferred for most cases
    /// due to its better performance and ability to run non-conflicting systems simultaneously.
    ///
    /// If you want to prevent Commands from registering the same system multiple times, consider using [`Local`](crate::system::Local)
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, world::CommandQueue, system::SystemId};
    ///
    /// #[derive(Resource)]
    /// struct Counter(i32);
    ///
    /// fn register_system(mut local_system: Local<Option<SystemId>>, mut commands: Commands) {
    ///     if let Some(system) = *local_system {
    ///         commands.run_system(system);
    ///     } else {
    ///         *local_system = Some(commands.register_system(increment_counter));
    ///     }
    /// }
    ///
    /// fn increment_counter(mut value: ResMut<Counter>) {
    ///     value.0 += 1;
    /// }
    ///
    /// # let mut world = World::default();
    /// # world.insert_resource(Counter(0));
    /// # let mut queue_1 = CommandQueue::default();
    /// # let systemid = {
    /// #   let mut commands = Commands::new(&mut queue_1, &world);
    /// #   commands.register_system(increment_counter)
    /// # };
    /// # let mut queue_2 = CommandQueue::default();
    /// # {
    /// #   let mut commands = Commands::new(&mut queue_2, &world);
    /// #   commands.run_system(systemid);
    /// # }
    /// # queue_1.append(&mut queue_2);
    /// # queue_1.apply(&mut world);
    /// # assert_eq!(1, world.resource::<Counter>().0);
    /// # bevy_ecs::system::assert_is_system(register_system);
    /// ```
    pub fn register_system<I, O, M>(
        &mut self,
        system: impl IntoSystem<I, O, M> + 'static,
    ) -> SystemId<I, O>
    where
        I: SystemInput + Send + 'static,
        O: Send + 'static,
    {
        let entity = self.spawn_empty().id();
        self.queue(RegisterSystem::new(system, entity));
        SystemId::from_entity(entity)
    }

    /// Removes a system previously registered with [`Commands::register_system`] or [`World::register_system`].
    ///
    /// See [`World::unregister_system`] for more information.
    pub fn unregister_system<I, O>(&mut self, system_id: SystemId<I, O>)
    where
        I: SystemInput + Send + 'static,
        O: Send + 'static,
    {
        self.queue(UnregisterSystem::new(system_id));
    }

    /// Removes a system previously registered with [`World::register_system_cached`].
    ///
    /// See [`World::unregister_system_cached`] for more information.
    pub fn unregister_system_cached<
        I: SystemInput + Send + 'static,
        O: 'static,
        M: 'static,
        S: IntoSystem<I, O, M> + Send + 'static,
    >(
        &mut self,
        system: S,
    ) {
        self.queue(UnregisterSystemCached::new(system));
    }

    /// Similar to [`Self::run_system`], but caching the [`SystemId`] in a
    /// [`CachedSystemId`](crate::system::CachedSystemId) resource.
    ///
    /// See [`World::register_system_cached`] for more information.
    pub fn run_system_cached<M: 'static, S: IntoSystem<(), (), M> + Send + 'static>(
        &mut self,
        system: S,
    ) {
        self.run_system_cached_with(system, ());
    }

    /// Similar to [`Self::run_system_with`], but caching the [`SystemId`] in a
    /// [`CachedSystemId`](crate::system::CachedSystemId) resource.
    ///
    /// See [`World::register_system_cached`] for more information.
    pub fn run_system_cached_with<I, M, S>(&mut self, system: S, input: I::Inner<'static>)
    where
        I: SystemInput<Inner<'static>: Send> + Send + 'static,
        M: 'static,
        S: IntoSystem<I, (), M> + Send + 'static,
    {
        self.queue(RunSystemCachedWith::new(system, input));
    }

    /// Sends a "global" [`Trigger`] without any targets. This will run any [`Observer`] of the `event` that
    /// isn't scoped to specific targets.
    ///
    /// [`Trigger`]: crate::observer::Trigger
    pub fn trigger(&mut self, event: impl Event) {
        self.queue(TriggerEvent { event, targets: () });
    }

    /// Sends a [`Trigger`] for the given targets. This will run any [`Observer`] of the `event` that
    /// watches those targets.
    ///
    /// [`Trigger`]: crate::observer::Trigger
    pub fn trigger_targets(
        &mut self,
        event: impl Event,
        targets: impl TriggerTargets + Send + Sync + 'static,
    ) {
        self.queue(TriggerEvent { event, targets });
    }

    /// Spawns an [`Observer`] and returns the [`EntityCommands`] associated
    /// with the entity that stores the observer.
    ///
    /// **Calling [`observe`](EntityCommands::observe) on the returned
    /// [`EntityCommands`] will observe the observer itself, which you very
    /// likely do not want.**
    pub fn add_observer<E: Event, B: Bundle, M>(
        &mut self,
        observer: impl IntoObserverSystem<E, B, M>,
    ) -> EntityCommands {
        self.spawn(Observer::new(observer))
    }

    /// Sends an arbitrary [`Event`].
    ///
    /// This is a convenience method for sending events without requiring an [`EventWriter`].
    /// ## Performance
    /// Since this is a command, exclusive world access is used, which means that it will not profit from
    /// system-level parallelism on supported platforms.
    /// If these events are performance-critical or very frequently
    /// sent, consider using a typed [`EventWriter`] instead.
    ///
    /// [`EventWriter`]: crate::event::EventWriter
    #[track_caller]
    pub fn send_event<E: Event>(&mut self, event: E) -> &mut Self {
        self.queue(SendEvent {
            event,
            #[cfg(feature = "track_change_detection")]
            caller: Location::caller(),
        });
        self
    }

    /// Runs the schedule corresponding to the given [`ScheduleLabel`].
    ///
    /// Calls [`World::try_run_schedule`](World::try_run_schedule).
    ///
    /// This will log an error if the schedule is not available to be run.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ecs::schedule::ScheduleLabel;
    /// #
    /// # #[derive(Default, Resource)]
    /// # struct Counter(u32);
    /// #
    /// #[derive(ScheduleLabel, Hash, Debug, PartialEq, Eq, Clone, Copy)]
    /// struct FooSchedule;
    ///
    /// # fn foo_system(mut counter: ResMut<Counter>) {
    /// #     counter.0 += 1;
    /// # }
    /// #
    /// # let mut schedule = Schedule::new(FooSchedule);
    /// # schedule.add_systems(foo_system);
    /// #
    /// # let mut world = World::default();
    /// #
    /// # world.init_resource::<Counter>();
    /// # world.add_schedule(schedule);
    /// #
    /// # assert_eq!(world.resource::<Counter>().0, 0);
    /// #
    /// # let mut commands = world.commands();
    /// commands.run_schedule(FooSchedule);
    /// #
    /// # world.flush();
    /// #
    /// # assert_eq!(world.resource::<Counter>().0, 1);
    /// ```
    pub fn run_schedule(&mut self, label: impl ScheduleLabel) {
        self.queue(|world: &mut World| {
            if let Err(error) = world.try_run_schedule(label) {
                panic!("Failed to run schedule: {error}");
            }
        });
    }
}

/// A [`Command`] which gets executed for a given [`Entity`].
///
/// # Examples
///
/// ```
/// # use std::collections::HashSet;
/// # use bevy_ecs::prelude::*;
/// use bevy_ecs::system::EntityCommand;
/// #
/// # #[derive(Component, PartialEq)]
/// # struct Name(String);
/// # impl Name {
/// #   fn new(s: String) -> Self { Name(s) }
/// #   fn as_str(&self) -> &str { &self.0 }
/// # }
///
/// #[derive(Resource, Default)]
/// struct Counter(i64);
///
/// /// A `Command` which names an entity based on a global counter.
/// fn count_name(entity: Entity, world: &mut World) {
///     // Get the current value of the counter, and increment it for next time.
///     let mut counter = world.resource_mut::<Counter>();
///     let i = counter.0;
///     counter.0 += 1;
///
///     // Name the entity after the value of the counter.
///     world.entity_mut(entity).insert(Name::new(format!("Entity #{i}")));
/// }
///
/// // App creation boilerplate omitted...
/// # let mut world = World::new();
/// # world.init_resource::<Counter>();
/// #
/// # let mut setup_schedule = Schedule::default();
/// # setup_schedule.add_systems(setup);
/// # let mut assert_schedule = Schedule::default();
/// # assert_schedule.add_systems(assert_names);
/// #
/// # setup_schedule.run(&mut world);
/// # assert_schedule.run(&mut world);
///
/// fn setup(mut commands: Commands) {
///     commands.spawn_empty().queue(count_name);
///     commands.spawn_empty().queue(count_name);
/// }
///
/// fn assert_names(named: Query<&Name>) {
///     // We use a HashSet because we do not care about the order.
///     let names: HashSet<_> = named.iter().map(Name::as_str).collect();
///     assert_eq!(names, HashSet::from_iter(["Entity #0", "Entity #1"]));
/// }
/// ```
pub trait EntityCommand<Marker = ()>: Send + 'static {
    /// Executes this command for the given [`Entity`].
    fn apply(self, entity: Entity, world: &mut World);

    /// Returns a [`Command`] which executes this [`EntityCommand`] for the given [`Entity`].
    ///
    /// This method is called when adding an [`EntityCommand`] to a command queue via [`Commands`].
    /// You can override the provided implementation if you can return a `Command` with a smaller memory
    /// footprint than `(Entity, Self)`.
    /// In most cases the provided implementation is sufficient.
    #[must_use = "commands do nothing unless applied to a `World`"]
    fn with_entity(self, entity: Entity) -> impl Command
    where
        Self: Sized,
    {
        move |world: &mut World| self.apply(entity, world)
    }
}

/// A list of commands that will be run to modify an [entity](crate::entity).
pub struct EntityCommands<'a> {
    pub(crate) entity: Entity,
    pub(crate) commands: Commands<'a, 'a>,
}

impl<'a> EntityCommands<'a> {
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

    /// Returns an [`EntityCommands`] with a smaller lifetime.
    /// This is useful if you have `&mut EntityCommands` but you need `EntityCommands`.
    pub fn reborrow(&mut self) -> EntityCommands {
        EntityCommands {
            entity: self.entity,
            commands: self.commands.reborrow(),
        }
    }

    /// Get an [`EntityEntryCommands`] for the [`Component`] `T`,
    /// allowing you to modify it or insert it if it isn't already present.
    ///
    /// See also [`insert_if_new`](Self::insert_if_new), which lets you insert a [`Bundle`] without overwriting it.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource)]
    /// # struct PlayerEntity { entity: Entity }
    /// #[derive(Component)]
    /// struct Level(u32);
    ///
    /// fn level_up_system(mut commands: Commands, player: Res<PlayerEntity>) {
    ///     commands
    ///         .entity(player.entity)
    ///         .entry::<Level>()
    ///         // Modify the component if it exists
    ///         .and_modify(|mut lvl| lvl.0 += 1)
    ///         // Otherwise insert a default value
    ///         .or_insert(Level(0));
    /// }
    /// # bevy_ecs::system::assert_is_system(level_up_system);
    /// ```
    pub fn entry<T: Component>(&mut self) -> EntityEntryCommands<T> {
        EntityEntryCommands {
            entity_commands: self.reborrow(),
            marker: PhantomData,
        }
    }

    /// Adds a [`Bundle`] of components to the entity.
    ///
    /// This will overwrite any previous value(s) of the same component type.
    /// See [`EntityCommands::insert_if_new`] to keep the old value instead.
    ///
    /// # Panics
    ///
    /// The command will panic when applied if the associated entity does not exist.
    ///
    /// To avoid a panic in this case, use the command [`Self::try_insert`] instead.
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
    #[track_caller]
    pub fn insert(&mut self, bundle: impl Bundle) -> &mut Self {
        self.queue(insert(bundle, InsertMode::Replace))
    }

    /// Similar to [`Self::insert`] but will only insert if the predicate returns true.
    /// This is useful for chaining method calls.
    ///
    /// # Panics
    ///
    /// The command will panic when applied if the associated entity does not exist.
    ///
    /// To avoid a panic in this case, use the command [`Self::try_insert_if`] instead.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource)]
    /// # struct PlayerEntity { entity: Entity }
    /// # impl PlayerEntity { fn is_spectator(&self) -> bool { true } }
    /// #[derive(Component)]
    /// struct StillLoadingStats;
    /// #[derive(Component)]
    /// struct Health(u32);
    ///
    /// fn add_health_system(mut commands: Commands, player: Res<PlayerEntity>) {
    ///     commands
    ///         .entity(player.entity)
    ///         .insert_if(Health(10), || !player.is_spectator())
    ///         .remove::<StillLoadingStats>();
    /// }
    /// # bevy_ecs::system::assert_is_system(add_health_system);
    /// ```
    #[track_caller]
    pub fn insert_if<F>(&mut self, bundle: impl Bundle, condition: F) -> &mut Self
    where
        F: FnOnce() -> bool,
    {
        if condition() {
            self.queue(insert(bundle, InsertMode::Replace))
        } else {
            self
        }
    }

    /// Adds a [`Bundle`] of components to the entity without overwriting.
    ///
    /// This is the same as [`EntityCommands::insert`], but in case of duplicate
    /// components will leave the old values instead of replacing them with new
    /// ones.
    ///
    /// See also [`entry`](Self::entry), which lets you modify a [`Component`] if it's present,
    /// as well as initialize it with a default value.
    ///
    /// # Panics
    ///
    /// The command will panic when applied if the associated entity does not exist.
    ///
    /// To avoid a panic in this case, use the command [`Self::try_insert_if_new`] instead.
    pub fn insert_if_new(&mut self, bundle: impl Bundle) -> &mut Self {
        self.queue(insert(bundle, InsertMode::Keep))
    }

    /// Adds a [`Bundle`] of components to the entity without overwriting if the
    /// predicate returns true.
    ///
    /// This is the same as [`EntityCommands::insert_if`], but in case of duplicate
    /// components will leave the old values instead of replacing them with new
    /// ones.
    ///
    /// # Panics
    ///
    /// The command will panic when applied if the associated entity does not
    /// exist.
    ///
    /// To avoid a panic in this case, use the command [`Self::try_insert_if_new`]
    /// instead.
    pub fn insert_if_new_and<F>(&mut self, bundle: impl Bundle, condition: F) -> &mut Self
    where
        F: FnOnce() -> bool,
    {
        if condition() {
            self.insert_if_new(bundle)
        } else {
            self
        }
    }

    /// Adds a dynamic component to an entity.
    ///
    /// See [`EntityWorldMut::insert_by_id`] for more information.
    ///
    /// # Panics
    ///
    /// The command will panic when applied if the associated entity does not exist.
    ///
    /// To avoid a panic in this case, use the command [`Self::try_insert_by_id`] instead.
    ///
    /// # Safety
    ///
    /// - [`ComponentId`] must be from the same world as `self`.
    /// - `T` must have the same layout as the one passed during `component_id` creation.
    #[track_caller]
    pub unsafe fn insert_by_id<T: Send + 'static>(
        &mut self,
        component_id: ComponentId,
        value: T,
    ) -> &mut Self {
        let caller = Location::caller();
        // SAFETY: same invariants as parent call
        self.queue(unsafe {insert_by_id(component_id, value, move |entity| {
            panic!("error[B0003]: {caller}: Could not insert a component {component_id:?} (with type {}) for entity {entity:?} because it doesn't exist in this World. See: https://bevyengine.org/learn/errors/b0003", core::any::type_name::<T>());
        })})
    }

    /// Attempts to add a dynamic component to an entity.
    ///
    /// See [`EntityWorldMut::insert_by_id`] for more information.
    ///
    /// # Safety
    ///
    /// - [`ComponentId`] must be from the same world as `self`.
    /// - `T` must have the same layout as the one passed during `component_id` creation.
    pub unsafe fn try_insert_by_id<T: Send + 'static>(
        &mut self,
        component_id: ComponentId,
        value: T,
    ) -> &mut Self {
        // SAFETY: same invariants as parent call
        self.queue(unsafe { insert_by_id(component_id, value, |_| {}) })
    }

    /// Tries to add a [`Bundle`] of components to the entity.
    ///
    /// This will overwrite any previous value(s) of the same component type.
    ///
    /// # Note
    ///
    /// Unlike [`Self::insert`], this will not panic if the associated entity does not exist.
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
    ///   commands.entity(player.entity)
    ///    // You can try_insert individual components:
    ///     .try_insert(Defense(10))
    ///
    ///    // You can also insert tuples of components:
    ///     .try_insert(CombatBundle {
    ///         health: Health(100),
    ///         strength: Strength(40),
    ///     });
    ///
    ///    // Suppose this occurs in a parallel adjacent system or process
    ///    commands.entity(player.entity)
    ///      .despawn();
    ///
    ///    commands.entity(player.entity)
    ///    // This will not panic nor will it add the component
    ///      .try_insert(Defense(5));
    /// }
    /// # bevy_ecs::system::assert_is_system(add_combat_stats_system);
    /// ```
    #[track_caller]
    pub fn try_insert(&mut self, bundle: impl Bundle) -> &mut Self {
        self.queue(try_insert(bundle, InsertMode::Replace))
    }

    /// Similar to [`Self::try_insert`] but will only try to insert if the predicate returns true.
    /// This is useful for chaining method calls.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource)]
    /// # struct PlayerEntity { entity: Entity }
    /// # impl PlayerEntity { fn is_spectator(&self) -> bool { true } }
    /// #[derive(Component)]
    /// struct StillLoadingStats;
    /// #[derive(Component)]
    /// struct Health(u32);
    ///
    /// fn add_health_system(mut commands: Commands, player: Res<PlayerEntity>) {
    ///   commands.entity(player.entity)
    ///     .try_insert_if(Health(10), || !player.is_spectator())
    ///     .remove::<StillLoadingStats>();
    ///
    ///    commands.entity(player.entity)
    ///    // This will not panic nor will it add the component
    ///      .try_insert_if(Health(5), || !player.is_spectator());
    /// }
    /// # bevy_ecs::system::assert_is_system(add_health_system);
    /// ```
    #[track_caller]
    pub fn try_insert_if<F>(&mut self, bundle: impl Bundle, condition: F) -> &mut Self
    where
        F: FnOnce() -> bool,
    {
        if condition() {
            self.queue(try_insert(bundle, InsertMode::Replace))
        } else {
            self
        }
    }

    /// Tries to add a [`Bundle`] of components to the entity without overwriting if the
    /// predicate returns true.
    ///
    /// This is the same as [`EntityCommands::try_insert_if`], but in case of duplicate
    /// components will leave the old values instead of replacing them with new
    /// ones.
    ///
    /// # Note
    ///
    /// Unlike [`Self::insert_if_new_and`], this will not panic if the associated entity does
    /// not exist.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource)]
    /// # struct PlayerEntity { entity: Entity }
    /// # impl PlayerEntity { fn is_spectator(&self) -> bool { true } }
    /// #[derive(Component)]
    /// struct StillLoadingStats;
    /// #[derive(Component)]
    /// struct Health(u32);
    ///
    /// fn add_health_system(mut commands: Commands, player: Res<PlayerEntity>) {
    ///   commands.entity(player.entity)
    ///     .try_insert_if(Health(10), || player.is_spectator())
    ///     .remove::<StillLoadingStats>();
    ///
    ///    commands.entity(player.entity)
    ///    // This will not panic nor will it overwrite the component
    ///      .try_insert_if_new_and(Health(5), || player.is_spectator());
    /// }
    /// # bevy_ecs::system::assert_is_system(add_health_system);
    /// ```
    pub fn try_insert_if_new_and<F>(&mut self, bundle: impl Bundle, condition: F) -> &mut Self
    where
        F: FnOnce() -> bool,
    {
        if condition() {
            self.try_insert_if_new(bundle)
        } else {
            self
        }
    }

    /// Tries to add a [`Bundle`] of components to the entity without overwriting.
    ///
    /// This is the same as [`EntityCommands::try_insert`], but in case of duplicate
    /// components will leave the old values instead of replacing them with new
    /// ones.
    ///
    /// # Note
    ///
    /// Unlike [`Self::insert_if_new`], this will not panic if the associated entity does not exist.
    pub fn try_insert_if_new(&mut self, bundle: impl Bundle) -> &mut Self {
        self.queue(try_insert(bundle, InsertMode::Keep))
    }

    /// Removes a [`Bundle`] of components from the entity.
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
        self.queue(remove::<T>)
    }

    /// Removes all components in the [`Bundle`] components and remove all required components for each component in the [`Bundle`] from entity.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// #[require(B)]
    /// struct A;
    /// #[derive(Component, Default)]
    /// struct B;
    ///
    /// #[derive(Resource)]
    /// struct PlayerEntity { entity: Entity }
    ///
    /// fn remove_with_requires_system(mut commands: Commands, player: Res<PlayerEntity>) {
    ///     commands
    ///         .entity(player.entity)
    ///         // Remove both A and B components from the entity, because B is required by A
    ///         .remove_with_requires::<A>();
    /// }
    /// # bevy_ecs::system::assert_is_system(remove_with_requires_system);
    /// ```
    pub fn remove_with_requires<T: Bundle>(&mut self) -> &mut Self {
        self.queue(remove_with_requires::<T>)
    }

    /// Removes a component from the entity.
    pub fn remove_by_id(&mut self, component_id: ComponentId) -> &mut Self {
        self.queue(remove_by_id(component_id))
    }

    /// Removes all components associated with the entity.
    pub fn clear(&mut self) -> &mut Self {
        self.queue(clear())
    }

    /// Despawns the entity.
    /// This will emit a warning if the entity does not exist.
    ///
    /// See [`World::despawn`] for more details.
    ///
    /// # Note
    ///
    /// This won't clean up external references to the entity (such as parent-child relationships
    /// if you're using `bevy_hierarchy`), which may leave the world in an invalid state.
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
    #[track_caller]
    pub fn despawn(&mut self) {
        self.queue(despawn());
    }

    /// Despawns the entity.
    /// This will not emit a warning if the entity does not exist, essentially performing
    /// the same function as [`Self::despawn`] without emitting warnings.
    #[track_caller]
    pub fn try_despawn(&mut self) {
        self.queue(try_despawn());
    }

    /// Pushes an [`EntityCommand`] to the queue, which will get executed for the current [`Entity`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # fn my_system(mut commands: Commands) {
    /// commands
    ///     .spawn_empty()
    ///     // Closures with this signature implement `EntityCommand`.
    ///     .queue(|entity: EntityWorldMut| {
    ///         println!("Executed an EntityCommand for {:?}", entity.id());
    ///     });
    /// # }
    /// # bevy_ecs::system::assert_is_system(my_system);
    /// ```
    pub fn queue<M: 'static>(&mut self, command: impl EntityCommand<M>) -> &mut Self {
        self.commands.queue(command.with_entity(self.entity));
        self
    }

    /// Removes all components except the given [`Bundle`] from the entity.
    ///
    /// This can also be used to remove all the components from the entity by passing it an empty Bundle.
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
    ///         // You can retain a pre-defined Bundle of components,
    ///         // with this removing only the Defense component
    ///         .retain::<CombatBundle>()
    ///         // You can also retain only a single component
    ///         .retain::<Health>()
    ///         // And you can remove all the components by passing in an empty Bundle
    ///         .retain::<()>();
    /// }
    /// # bevy_ecs::system::assert_is_system(remove_combat_stats_system);
    /// ```
    pub fn retain<T>(&mut self) -> &mut Self
    where
        T: Bundle,
    {
        self.queue(retain::<T>)
    }

    /// Logs the components of the entity at the info level.
    ///
    /// # Panics
    ///
    /// The command will panic when applied if the associated entity does not exist.
    pub fn log_components(&mut self) -> &mut Self {
        self.queue(log_components)
    }

    /// Returns the underlying [`Commands`].
    pub fn commands(&mut self) -> Commands {
        self.commands.reborrow()
    }

    /// Returns a mutable reference to the underlying [`Commands`].
    pub fn commands_mut(&mut self) -> &mut Commands<'a, 'a> {
        &mut self.commands
    }

    /// Sends a [`Trigger`] targeting this entity. This will run any [`Observer`] of the `event` that
    /// watches this entity.
    ///
    /// [`Trigger`]: crate::observer::Trigger
    pub fn trigger(&mut self, event: impl Event) -> &mut Self {
        self.commands.trigger_targets(event, self.entity);
        self
    }

    /// Creates an [`Observer`] listening for a trigger of type `T` that targets this entity.
    pub fn observe<E: Event, B: Bundle, M>(
        &mut self,
        system: impl IntoObserverSystem<E, B, M>,
    ) -> &mut Self {
        self.queue(observe(system))
    }

    /// Clones an entity and returns the [`EntityCommands`] of the clone.
    ///
    /// The clone will receive all the components of the original that implement
    /// [`Clone`] or [`Reflect`](bevy_reflect::Reflect).
    ///
    /// To configure cloning behavior (such as only cloning certain components),
    /// use [`EntityCommands::clone_and_spawn_with`].
    ///
    /// # Panics
    ///
    /// The command will panic when applied if the original entity does not exist.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    ///
    /// #[derive(Component, Clone)]
    /// struct ComponentA(u32);
    /// #[derive(Component, Clone)]
    /// struct ComponentB(u32);
    ///
    /// fn example_system(mut commands: Commands) {
    ///     // Create a new entity and keep its EntityCommands
    ///     let mut entity = commands.spawn((ComponentA(10), ComponentB(20)));
    ///
    ///     // Create a clone of the first entity
    ///     let mut entity_clone = entity.clone_and_spawn();
    /// }
    /// # bevy_ecs::system::assert_is_system(example_system);
    pub fn clone_and_spawn(&mut self) -> EntityCommands<'_> {
        self.clone_and_spawn_with(|_| {})
    }

    /// Clones an entity and allows configuring cloning behavior using [`EntityCloneBuilder`],
    /// returning the [`EntityCommands`] of the clone.
    ///
    /// By default, the clone will receive all the components of the original that implement
    /// [`Clone`] or [`Reflect`](bevy_reflect::Reflect).
    ///
    /// To exclude specific components, use [`EntityCloneBuilder::deny`].
    /// To only include specific components, use [`EntityCloneBuilder::deny_all`]
    /// followed by [`EntityCloneBuilder::allow`].
    ///
    /// # Panics
    ///
    /// The command will panic when applied if the original entity does not exist.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    ///
    /// #[derive(Component, Clone)]
    /// struct ComponentA(u32);
    /// #[derive(Component, Clone)]
    /// struct ComponentB(u32);
    ///
    /// fn example_system(mut commands: Commands) {
    ///     // Create a new entity and keep its EntityCommands
    ///     let mut entity = commands.spawn((ComponentA(10), ComponentB(20)));
    ///
    ///     // Create a clone of the first entity, but without ComponentB
    ///     let mut entity_clone = entity.clone_and_spawn_with(|builder| {
    ///         builder.deny::<ComponentB>();
    ///     });
    /// }
    /// # bevy_ecs::system::assert_is_system(example_system);
    pub fn clone_and_spawn_with(
        &mut self,
        f: impl FnOnce(&mut EntityCloneBuilder) + Send + Sync + 'static,
    ) -> EntityCommands<'_> {
        let entity_clone = self.commands().spawn_empty().id();
        self.queue(clone_and_spawn_with(entity_clone, f));
        EntityCommands {
            commands: self.commands_mut().reborrow(),
            entity: entity_clone,
        }
    }
}

/// A wrapper around [`EntityCommands`] with convenience methods for working with a specified component type.
pub struct EntityEntryCommands<'a, T> {
    entity_commands: EntityCommands<'a>,
    marker: PhantomData<T>,
}

impl<'a, T: Component<Mutability = Mutable>> EntityEntryCommands<'a, T> {
    /// Modify the component `T` if it exists, using the function `modify`.
    pub fn and_modify(&mut self, modify: impl FnOnce(Mut<T>) + Send + Sync + 'static) -> &mut Self {
        self.entity_commands
            .queue(move |mut entity: EntityWorldMut| {
                if let Some(value) = entity.get_mut() {
                    modify(value);
                }
            });
        self
    }
}

impl<'a, T: Component> EntityEntryCommands<'a, T> {
    /// [Insert](EntityCommands::insert) `default` into this entity, if `T` is not already present.
    ///
    /// See also [`or_insert_with`](Self::or_insert_with).
    ///
    /// # Panics
    ///
    /// Panics if the entity does not exist.
    /// See [`or_try_insert`](Self::or_try_insert) for a non-panicking version.
    #[track_caller]
    pub fn or_insert(&mut self, default: T) -> &mut Self {
        self.entity_commands
            .queue(insert(default, InsertMode::Keep));
        self
    }

    /// [Insert](EntityCommands::insert) `default` into this entity, if `T` is not already present.
    ///
    /// Unlike [`or_insert`](Self::or_insert), this will not panic if the entity does not exist.
    ///
    /// See also [`or_insert_with`](Self::or_insert_with).
    #[track_caller]
    pub fn or_try_insert(&mut self, default: T) -> &mut Self {
        self.entity_commands
            .queue(try_insert(default, InsertMode::Keep));
        self
    }

    /// [Insert](EntityCommands::insert) the value returned from `default` into this entity, if `T` is not already present.
    ///
    /// See also [`or_insert`](Self::or_insert) and [`or_try_insert`](Self::or_try_insert).
    ///
    /// # Panics
    ///
    /// Panics if the entity does not exist.
    /// See [`or_try_insert_with`](Self::or_try_insert_with) for a non-panicking version.
    #[track_caller]
    pub fn or_insert_with(&mut self, default: impl Fn() -> T) -> &mut Self {
        self.or_insert(default())
    }

    /// [Insert](EntityCommands::insert) the value returned from `default` into this entity, if `T` is not already present.
    ///
    /// Unlike [`or_insert_with`](Self::or_insert_with), this will not panic if the entity does not exist.
    ///
    /// See also [`or_insert`](Self::or_insert) and [`or_try_insert`](Self::or_try_insert).
    #[track_caller]
    pub fn or_try_insert_with(&mut self, default: impl Fn() -> T) -> &mut Self {
        self.or_try_insert(default())
    }

    /// [Insert](EntityCommands::insert) `T::default` into this entity, if `T` is not already present.
    ///
    /// See also [`or_insert`](Self::or_insert) and [`or_from_world`](Self::or_from_world).
    ///
    /// # Panics
    ///
    /// Panics if the entity does not exist.
    #[track_caller]
    pub fn or_default(&mut self) -> &mut Self
    where
        T: Default,
    {
        #[allow(clippy::unwrap_or_default)]
        // FIXME: use `expect` once stable
        self.or_insert(T::default())
    }

    /// [Insert](EntityCommands::insert) `T::from_world` into this entity, if `T` is not already present.
    ///
    /// See also [`or_insert`](Self::or_insert) and [`or_default`](Self::or_default).
    ///
    /// # Panics
    ///
    /// Panics if the entity does not exist.
    #[track_caller]
    pub fn or_from_world(&mut self) -> &mut Self
    where
        T: FromWorld,
    {
        self.entity_commands
            .queue(insert_from_world::<T>(InsertMode::Keep));
        self
    }
}

impl<F> Command for F
where
    F: FnOnce(&mut World) + Send + 'static,
{
    fn apply(self, world: &mut World) {
        self(world);
    }
}

impl<F> EntityCommand<World> for F
where
    F: FnOnce(EntityWorldMut) + Send + 'static,
{
    fn apply(self, id: Entity, world: &mut World) {
        self(world.entity_mut(id));
    }
}

impl<F> EntityCommand for F
where
    F: FnOnce(Entity, &mut World) + Send + 'static,
{
    fn apply(self, id: Entity, world: &mut World) {
        self(id, world);
    }
}

/// A [`Command`] that consumes an iterator of [`Bundle`]s to spawn a series of entities.
///
/// This is more efficient than spawning the entities individually.
#[track_caller]
fn spawn_batch<I, B>(bundles_iter: I) -> impl Command
where
    I: IntoIterator<Item = B> + Send + Sync + 'static,
    B: Bundle,
{
    #[cfg(feature = "track_change_detection")]
    let caller = Location::caller();
    move |world: &mut World| {
        SpawnBatchIter::new(
            world,
            bundles_iter.into_iter(),
            #[cfg(feature = "track_change_detection")]
            caller,
        );
    }
}

/// A [`Command`] that consumes an iterator to add a series of [`Bundle`]s to a set of entities.
/// If any entities do not already exist in the world, they will be spawned.
///
/// This is more efficient than inserting the bundles individually.
#[track_caller]
fn insert_or_spawn_batch<I, B>(bundles_iter: I) -> impl Command
where
    I: IntoIterator<Item = (Entity, B)> + Send + Sync + 'static,
    B: Bundle,
{
    #[cfg(feature = "track_change_detection")]
    let caller = Location::caller();
    move |world: &mut World| {
        if let Err(invalid_entities) = world.insert_or_spawn_batch_with_caller(
            bundles_iter,
            #[cfg(feature = "track_change_detection")]
            caller,
        ) {
            error!(
                "Failed to 'insert or spawn' bundle of type {} into the following invalid entities: {:?}",
                core::any::type_name::<B>(),
                invalid_entities
            );
        }
    }
}

/// A [`Command`] that consumes an iterator to add a series of [`Bundles`](Bundle) to a set of entities.
/// If any entities do not exist in the world, this command will panic.
///
/// This is more efficient than inserting the bundles individually.
#[track_caller]
fn insert_batch<I, B>(batch: I) -> impl Command
where
    I: IntoIterator<Item = (Entity, B)> + Send + Sync + 'static,
    B: Bundle,
{
    #[cfg(feature = "track_change_detection")]
    let caller = Location::caller();
    move |world: &mut World| {
        world.insert_batch_with_caller(
            batch,
            InsertMode::Replace,
            #[cfg(feature = "track_change_detection")]
            caller,
        );
    }
}

/// A [`Command`] that consumes an iterator to add a series of [`Bundles`](Bundle) to a set of entities.
/// If any entities do not exist in the world, this command will panic.
///
/// This is more efficient than inserting the bundles individually.
#[track_caller]
fn insert_batch_if_new<I, B>(batch: I) -> impl Command
where
    I: IntoIterator<Item = (Entity, B)> + Send + Sync + 'static,
    B: Bundle,
{
    #[cfg(feature = "track_change_detection")]
    let caller = Location::caller();
    move |world: &mut World| {
        world.insert_batch_with_caller(
            batch,
            InsertMode::Keep,
            #[cfg(feature = "track_change_detection")]
            caller,
        );
    }
}

/// A [`Command`] that consumes an iterator to add a series of [`Bundles`](Bundle) to a set of entities.
/// If any entities do not exist in the world, this command will ignore them.
///
/// This is more efficient than inserting the bundles individually.
#[track_caller]
fn try_insert_batch<I, B>(batch: I) -> impl Command
where
    I: IntoIterator<Item = (Entity, B)> + Send + Sync + 'static,
    B: Bundle,
{
    #[cfg(feature = "track_change_detection")]
    let caller = Location::caller();
    move |world: &mut World| {
        world.try_insert_batch_with_caller(
            batch,
            InsertMode::Replace,
            #[cfg(feature = "track_change_detection")]
            caller,
        );
    }
}

/// A [`Command`] that consumes an iterator to add a series of [`Bundles`](Bundle) to a set of entities.
/// If any entities do not exist in the world, this command will ignore them.
///
/// This is more efficient than inserting the bundles individually.
#[track_caller]
fn try_insert_batch_if_new<I, B>(batch: I) -> impl Command
where
    I: IntoIterator<Item = (Entity, B)> + Send + Sync + 'static,
    B: Bundle,
{
    #[cfg(feature = "track_change_detection")]
    let caller = Location::caller();
    move |world: &mut World| {
        world.try_insert_batch_with_caller(
            batch,
            InsertMode::Keep,
            #[cfg(feature = "track_change_detection")]
            caller,
        );
    }
}

/// A [`Command`] that despawns a specific entity.
/// This will emit a warning if the entity does not exist.
///
/// # Note
///
/// This won't clean up external references to the entity (such as parent-child relationships
/// if you're using `bevy_hierarchy`), which may leave the world in an invalid state.
#[track_caller]
fn despawn() -> impl EntityCommand {
    let caller = Location::caller();
    move |entity: Entity, world: &mut World| {
        world.despawn_with_caller(entity, caller, true);
    }
}

/// A [`Command`] that despawns a specific entity.
/// This will not emit a warning if the entity does not exist.
///
/// # Note
///
/// This won't clean up external references to the entity (such as parent-child relationships
/// if you're using `bevy_hierarchy`), which may leave the world in an invalid state.
#[track_caller]
fn try_despawn() -> impl EntityCommand {
    let caller = Location::caller();
    move |entity: Entity, world: &mut World| {
        world.despawn_with_caller(entity, caller, false);
    }
}

/// An [`EntityCommand`] that adds the components in a [`Bundle`] to an entity.
#[track_caller]
fn insert<T: Bundle>(bundle: T, mode: InsertMode) -> impl EntityCommand {
    let caller = Location::caller();
    move |entity: Entity, world: &mut World| {
        if let Ok(mut entity) = world.get_entity_mut(entity) {
            entity.insert_with_caller(
                bundle,
                mode,
                #[cfg(feature = "track_change_detection")]
                caller,
            );
        } else {
            panic!("error[B0003]: {caller}: Could not insert a bundle (of type `{}`) for entity {:?} because it doesn't exist in this World. See: https://bevyengine.org/learn/errors/b0003", core::any::type_name::<T>(), entity);
        }
    }
}

/// An [`EntityCommand`] that adds the component using its `FromWorld` implementation.
#[track_caller]
fn insert_from_world<T: Component + FromWorld>(mode: InsertMode) -> impl EntityCommand {
    let caller = Location::caller();
    move |entity: Entity, world: &mut World| {
        let value = T::from_world(world);
        if let Ok(mut entity) = world.get_entity_mut(entity) {
            entity.insert_with_caller(
                value,
                mode,
                #[cfg(feature = "track_change_detection")]
                caller,
            );
        } else {
            panic!("error[B0003]: {caller}: Could not insert a bundle (of type `{}`) for entity {:?} because it doesn't exist in this World. See: https://bevyengine.org/learn/errors/b0003", core::any::type_name::<T>(), entity);
        }
    }
}

/// An [`EntityCommand`] that attempts to add the components in a [`Bundle`] to an entity.
/// Does nothing if the entity does not exist.
#[track_caller]
fn try_insert(bundle: impl Bundle, mode: InsertMode) -> impl EntityCommand {
    #[cfg(feature = "track_change_detection")]
    let caller = Location::caller();
    move |entity: Entity, world: &mut World| {
        if let Ok(mut entity) = world.get_entity_mut(entity) {
            entity.insert_with_caller(
                bundle,
                mode,
                #[cfg(feature = "track_change_detection")]
                caller,
            );
        }
    }
}

/// An [`EntityCommand`] that attempts to add the dynamic component to an entity.
///
/// # Safety
///
/// - The returned `EntityCommand` must be queued for the world where `component_id` was created.
/// - `T` must be the type represented by `component_id`.
unsafe fn insert_by_id<T: Send + 'static>(
    component_id: ComponentId,
    value: T,
    on_none_entity: impl FnOnce(Entity) + Send + 'static,
) -> impl EntityCommand {
    move |entity: Entity, world: &mut World| {
        if let Ok(mut entity) = world.get_entity_mut(entity) {
            // SAFETY:
            // - `component_id` safety is ensured by the caller
            // - `ptr` is valid within the `make` block;
            OwningPtr::make(value, |ptr| unsafe {
                entity.insert_by_id(component_id, ptr);
            });
        } else {
            on_none_entity(entity);
        }
    }
}

/// An [`EntityCommand`] that removes components from an entity.
///
/// For a [`Bundle`] type `T`, this will remove any components in the bundle.
/// Any components in the bundle that aren't found on the entity will be ignored.
fn remove<T: Bundle>(entity: Entity, world: &mut World) {
    if let Ok(mut entity) = world.get_entity_mut(entity) {
        entity.remove::<T>();
    }
}

/// An [`EntityCommand`] that removes components with a provided [`ComponentId`] from an entity.
/// # Panics
///
/// Panics if the provided [`ComponentId`] does not exist in the [`World`].
fn remove_by_id(component_id: ComponentId) -> impl EntityCommand {
    move |entity: Entity, world: &mut World| {
        if let Ok(mut entity) = world.get_entity_mut(entity) {
            entity.remove_by_id(component_id);
        }
    }
}

/// An [`EntityCommand`] that remove all components in the bundle and remove all required components for each component in the bundle.
fn remove_with_requires<T: Bundle>(entity: Entity, world: &mut World) {
    if let Ok(mut entity) = world.get_entity_mut(entity) {
        entity.remove_with_requires::<T>();
    }
}

/// An [`EntityCommand`] that removes all components associated with a provided entity.
fn clear() -> impl EntityCommand {
    move |entity: Entity, world: &mut World| {
        if let Ok(mut entity) = world.get_entity_mut(entity) {
            entity.clear();
        }
    }
}

/// An [`EntityCommand`] that removes components from an entity.
///
/// For a [`Bundle`] type `T`, this will remove all components except those in the bundle.
/// Any components in the bundle that aren't found on the entity will be ignored.
fn retain<T: Bundle>(entity: Entity, world: &mut World) {
    if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
        entity_mut.retain::<T>();
    }
}

/// A [`Command`] that inserts a [`Resource`] into the world using a value
/// created with the [`FromWorld`] trait.
#[track_caller]
fn init_resource<R: Resource + FromWorld>(world: &mut World) {
    world.init_resource::<R>();
}

/// A [`Command`] that removes the [resource](Resource) `R` from the world.
#[track_caller]
fn remove_resource<R: Resource>(world: &mut World) {
    world.remove_resource::<R>();
}

/// A [`Command`] that inserts a [`Resource`] into the world.
#[track_caller]
fn insert_resource<R: Resource>(resource: R) -> impl Command {
    #[cfg(feature = "track_change_detection")]
    let caller = Location::caller();
    move |world: &mut World| {
        world.insert_resource_with_caller(
            resource,
            #[cfg(feature = "track_change_detection")]
            caller,
        );
    }
}

/// [`EntityCommand`] to log the components of a given entity. See [`EntityCommands::log_components`].
fn log_components(entity: Entity, world: &mut World) {
    let debug_infos: Vec<_> = world
        .inspect_entity(entity)
        .map(ComponentInfo::name)
        .collect();
    info!("Entity {entity}: {debug_infos:?}");
}

fn observe<E: Event, B: Bundle, M>(
    observer: impl IntoObserverSystem<E, B, M>,
) -> impl EntityCommand {
    move |entity: Entity, world: &mut World| {
        if let Ok(mut entity) = world.get_entity_mut(entity) {
            entity.observe(observer);
        }
    }
}

fn clone_and_spawn_with(
    entity_clone: Entity,
    f: impl FnOnce(&mut EntityCloneBuilder) + Send + Sync + 'static,
) -> impl EntityCommand {
    move |entity: Entity, world: &mut World| {
        let mut builder = EntityCloneBuilder::new(world);
        f(&mut builder);
        builder.clone_entity(entity, entity_clone);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use crate::{
        self as bevy_ecs,
        component::{require, Component},
        system::{Commands, Resource},
        world::{CommandQueue, FromWorld, World},
    };
    use alloc::sync::Arc;
    use core::{
        any::TypeId,
        sync::atomic::{AtomicUsize, Ordering},
    };

    #[allow(dead_code)]
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

    impl FromWorld for W<String> {
        fn from_world(world: &mut World) -> Self {
            let v = world.resource::<W<usize>>();
            Self("*".repeat(v.0))
        }
    }

    #[test]
    fn entity_commands_entry() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let entity = commands.spawn_empty().id();
        commands
            .entity(entity)
            .entry::<W<u32>>()
            .and_modify(|_| unreachable!());
        queue.apply(&mut world);
        assert!(!world.entity(entity).contains::<W<u32>>());
        let mut commands = Commands::new(&mut queue, &world);
        commands
            .entity(entity)
            .entry::<W<u32>>()
            .or_insert(W(0))
            .and_modify(|mut val| {
                val.0 = 21;
            });
        queue.apply(&mut world);
        assert_eq!(21, world.get::<W<u32>>(entity).unwrap().0);
        let mut commands = Commands::new(&mut queue, &world);
        commands
            .entity(entity)
            .entry::<W<u64>>()
            .and_modify(|_| unreachable!())
            .or_insert(W(42));
        queue.apply(&mut world);
        assert_eq!(42, world.get::<W<u64>>(entity).unwrap().0);
        world.insert_resource(W(5_usize));
        let mut commands = Commands::new(&mut queue, &world);
        commands.entity(entity).entry::<W<String>>().or_from_world();
        queue.apply(&mut world);
        assert_eq!("*****", &world.get::<W<String>>(entity).unwrap().0);
    }

    #[test]
    fn commands() {
        let mut world = World::default();
        let mut command_queue = CommandQueue::default();
        let entity = Commands::new(&mut command_queue, &world)
            .spawn((W(1u32), W(2u64)))
            .id();
        command_queue.apply(&mut world);
        assert_eq!(world.entities().len(), 1);
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
            commands.queue(|world: &mut World| {
                world.spawn((W(42u32), W(0u64)));
            });

            // set up a simple command using a function that adds one additional entity
            commands.queue(simple_command);
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
    fn insert_components() {
        let mut world = World::default();
        let mut command_queue1 = CommandQueue::default();

        // insert components
        let entity = Commands::new(&mut command_queue1, &world)
            .spawn(())
            .insert_if(W(1u8), || true)
            .insert_if(W(2u8), || false)
            .insert_if_new(W(1u16))
            .insert_if_new(W(2u16))
            .insert_if_new_and(W(1u32), || false)
            .insert_if_new_and(W(2u32), || true)
            .insert_if_new_and(W(3u32), || true)
            .id();
        command_queue1.apply(&mut world);

        let results = world
            .query::<(&W<u8>, &W<u16>, &W<u32>)>()
            .iter(&world)
            .map(|(a, b, c)| (a.0, b.0, c.0))
            .collect::<Vec<_>>();
        assert_eq!(results, vec![(1u8, 1u16, 2u32)]);

        // try to insert components after despawning entity
        // in another command queue
        Commands::new(&mut command_queue1, &world)
            .entity(entity)
            .try_insert_if_new_and(W(1u64), || true);

        let mut command_queue2 = CommandQueue::default();
        Commands::new(&mut command_queue2, &world)
            .entity(entity)
            .despawn();
        command_queue2.apply(&mut world);
        command_queue1.apply(&mut world);
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
    fn remove_components_by_id() {
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
            .remove_by_id(world.components().get_id(TypeId::of::<W<u32>>()).unwrap())
            .remove_by_id(world.components().get_id(TypeId::of::<W<u64>>()).unwrap())
            .remove_by_id(world.components().get_id(TypeId::of::<DropCk>()).unwrap())
            .remove_by_id(
                world
                    .components()
                    .get_id(TypeId::of::<SparseDropCk>())
                    .unwrap(),
            );

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

    #[test]
    fn remove_component_with_required_components() {
        #[derive(Component)]
        #[require(Y)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        #[derive(Component)]
        struct Z;

        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let e = {
            let mut commands = Commands::new(&mut queue, &world);
            commands.spawn((X, Z)).id()
        };
        queue.apply(&mut world);

        assert!(world.get::<Y>(e).is_some());
        assert!(world.get::<X>(e).is_some());
        assert!(world.get::<Z>(e).is_some());

        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(e).remove_with_requires::<X>();
        }
        queue.apply(&mut world);

        assert!(world.get::<Y>(e).is_none());
        assert!(world.get::<X>(e).is_none());

        assert!(world.get::<Z>(e).is_some());
    }

    #[test]
    fn unregister_system_cached_commands() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();

        fn nothing() {}

        assert!(world.iter_resources().count() == 0);
        let id = world.register_system_cached(nothing);
        assert!(world.iter_resources().count() == 1);
        assert!(world.get_entity(id.entity).is_ok());

        let mut commands = Commands::new(&mut queue, &world);
        commands.unregister_system_cached(nothing);
        queue.apply(&mut world);
        assert!(world.iter_resources().count() == 0);
        assert!(world.get_entity(id.entity).is_err());
    }

    fn is_send<T: Send>() {}
    fn is_sync<T: Sync>() {}

    #[test]
    fn test_commands_are_send_and_sync() {
        is_send::<Commands>();
        is_sync::<Commands>();
    }

    #[test]
    fn append() {
        let mut world = World::default();
        let mut queue_1 = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue_1, &world);
            commands.insert_resource(W(123i32));
        }
        let mut queue_2 = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue_2, &world);
            commands.insert_resource(W(456.0f64));
        }
        queue_1.append(&mut queue_2);
        queue_1.apply(&mut world);
        assert!(world.contains_resource::<W<i32>>());
        assert!(world.contains_resource::<W<f64>>());
    }
}
