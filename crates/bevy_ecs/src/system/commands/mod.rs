pub mod command;
pub mod entity_command;

#[cfg(feature = "std")]
mod parallel_scope;

use bevy_ptr::move_as_ptr;
pub use command::Command;
pub use entity_command::EntityCommand;

#[cfg(feature = "std")]
pub use parallel_scope::*;

use alloc::boxed::Box;
use bevy_platform::prelude::*;
use bevy_platform::sync::OnceLock;
use core::marker::PhantomData;

use crate::{
    self as bevy_ecs,
    bundle::{Bundle, InsertMode, NoBundleEffect},
    change_detection::{MaybeLocation, Mut},
    component::{Component, ComponentId, Mutable},
    entity::{
        Entities, Entity, EntityAllocator, EntityClonerBuilder, EntityNotSpawnedError,
        InvalidEntityError, OptIn, OptOut,
    },
    error::{warn, BevyError, CommandWithEntity, ErrorContext, HandleError},
    event::{EntityEvent, Event},
    message::Message,
    observer::{IntoEntityObserver, IntoObserver},
    resource::Resource,
    schedule::ScheduleLabel,
    system::{
        Deferred, IntoSystem, RegisteredSystem, SharedState, SharedStateVTable, SharedStates,
        SystemId, SystemInput, SystemParam, SystemParamValidationError,
    },
    world::{
        command_queue::RawCommandQueue, unsafe_world_cell::UnsafeWorldCell, CommandQueue,
        EntityWorldMut, FromWorld, World,
    },
};

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
/// Commands are almost always used as a [`SystemParam`].
///
/// ```
/// # use bevy_ecs::prelude::*;
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
/// });
/// # }
/// ```
///
/// # Error handling
///
/// A [`Command`] can return a [`Result`](crate::error::Result),
/// which will be passed to an [error handler](crate::error) if the `Result` is an error.
///
/// The default error handler panics. It can be configured via
/// the [`DefaultErrorHandler`](crate::error::DefaultErrorHandler) resource.
///
/// Alternatively, you can customize the error handler for a specific command
/// by calling [`Commands::queue_handled`].
///
/// The [`error`](crate::error) module provides some simple error handlers for convenience.
///
/// [`ApplyDeferred`]: crate::schedule::ApplyDeferred
pub struct Commands<'w, 's> {
    queue: InternalQueue<'s>,
    entities: &'w Entities,
    allocator: &'w EntityAllocator,
}

// SAFETY: All commands [`Command`] implement [`Send`]
unsafe impl Send for Commands<'_, '_> {}

// SAFETY: `Commands` never gives access to the inner commands.
unsafe impl Sync for Commands<'_, '_> {}

const _: () = {
    type __StructFieldsAlias<'w, 's> = (&'w EntityAllocator, &'w Entities);
    #[doc(hidden)]
    pub struct FetchState {
        state: <__StructFieldsAlias<'static, 'static> as SystemParam>::State,
    }
    // SAFETY: Only reads Entities
    unsafe impl SystemParam for Commands<'_, '_> {
        type State = (SharedState<CommandQueue>, FetchState);

        type Item<'w, 's> = Commands<'w, 's>;

        fn shared() -> &'static [&'static SharedStateVTable] {
            static VTABLE: OnceLock<&'static [&'static SharedStateVTable]> = OnceLock::new();
            VTABLE.get_or_init(|| vec![SharedStateVTable::of::<CommandQueue>()].leak())
        }

        #[track_caller]
        unsafe fn init_state(world: &mut World, shared_states: &SharedStates) -> Self::State {
            // SAFETY: requirements upheld by caller
            unsafe {
                (
                    SharedState::new(shared_states)
                        .expect("CommandQueue should be initialized in SharedStates"),
                    FetchState {
                        state: __StructFieldsAlias::init_state(world, shared_states),
                    },
                )
            }
        }

        fn init_access(
            state: &Self::State,
            system_meta: &mut bevy_ecs::system::SystemMeta,
            component_access_set: &mut bevy_ecs::query::FilteredAccessSet,
            world: &mut World,
        ) {
            <__StructFieldsAlias<'_, '_> as SystemParam>::init_access(
                &state.1.state,
                system_meta,
                component_access_set,
                world,
            );
        }

        fn apply(
            state: &mut Self::State,
            system_meta: &bevy_ecs::system::SystemMeta,
            world: &mut World,
        ) {
            <__StructFieldsAlias<'_, '_> as SystemParam>::apply(
                &mut state.1.state,
                system_meta,
                world,
            );
        }

        fn queue(
            state: &mut Self::State,
            system_meta: &bevy_ecs::system::SystemMeta,
            world: bevy_ecs::world::DeferredWorld,
        ) {
            <__StructFieldsAlias<'_, '_> as SystemParam>::queue(
                &mut state.1.state,
                system_meta,
                world,
            );
        }

        #[inline]
        unsafe fn validate_param(
            state: &mut Self::State,
            system_meta: &bevy_ecs::system::SystemMeta,
            world: UnsafeWorldCell,
        ) -> Result<(), SystemParamValidationError> {
            // SAFETY: Upheld by caller
            unsafe {
                <__StructFieldsAlias as SystemParam>::validate_param(
                    &mut state.1.state,
                    system_meta,
                    world,
                )
            }
        }

        #[inline]
        #[track_caller]
        unsafe fn get_param<'w, 's>(
            state: &'s mut Self::State,
            system_meta: &bevy_ecs::system::SystemMeta,
            world: UnsafeWorldCell<'w>,
            change_tick: bevy_ecs::change_detection::Tick,
        ) -> Self::Item<'w, 's> {
            // SAFETY: Upheld by caller
            let params = unsafe {
                <__StructFieldsAlias as SystemParam>::get_param(
                    &mut state.1.state,
                    system_meta,
                    world,
                    change_tick,
                )
            };
            Commands {
                // SAFETY: `Commands` is not `Send` or `Sync` so there is no way for more than one
                //          of them to be using the command queue at once
                queue: unsafe { InternalQueue::RawCommandQueue(state.0.get_raw()) },
                allocator: params.0,
                entities: params.1,
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
    pub fn new(queue: &'s mut CommandQueue, world: &'w World) -> Self {
        Self::new_from_entities(queue, &world.entity_allocator, &world.entities)
    }

    /// Returns a new `Commands` instance from a [`CommandQueue`] and an [`Entities`] reference.
    pub fn new_from_entities(
        queue: &'s mut CommandQueue,
        allocator: &'w EntityAllocator,
        entities: &'w Entities,
    ) -> Self {
        Self {
            queue: InternalQueue::CommandQueue(Deferred(queue)),
            allocator,
            entities,
        }
    }

    /// Returns a new `Commands` instance from a [`RawCommandQueue`] and an [`Entities`] reference.
    ///
    /// This is used when constructing [`Commands`] from a [`DeferredWorld`](crate::world::DeferredWorld).
    ///
    /// # Safety
    ///
    /// * Caller ensures that `queue` must outlive `'w`
    pub(crate) unsafe fn new_raw_from_entities(
        queue: RawCommandQueue,
        allocator: &'w EntityAllocator,
        entities: &'w Entities,
    ) -> Self {
        Self {
            queue: InternalQueue::RawCommandQueue(queue),
            allocator,
            entities,
        }
    }

    /// Returns a [`Commands`] with a smaller lifetime.
    ///
    /// This is useful if you have `&mut Commands` but need `Commands`.
    ///
    /// # Example
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
            allocator: self.allocator,
            entities: self.entities,
        }
    }

    /// Take all commands from `other` and append them to `self`, leaving `other` empty.
    pub fn append(&mut self, other: &mut CommandQueue) {
        match &mut self.queue {
            InternalQueue::CommandQueue(queue) => {
                queue.bytes.get_mut().append(other.bytes.get_mut());
            }
            InternalQueue::RawCommandQueue(queue) => {
                // SAFETY: Pointers in `RawCommandQueue` are never null
                unsafe { queue.bytes.as_mut() }.append(other.bytes.get_mut());
            }
        }
    }

    /// Spawns a new empty [`Entity`] and returns its corresponding [`EntityCommands`].
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component)]
    /// struct Label(&'static str);
    /// #[derive(Component)]
    /// struct Strength(u32);
    /// #[derive(Component)]
    /// struct Agility(u32);
    ///
    /// fn example_system(mut commands: Commands) {
    ///     // Create a new empty entity.
    ///     commands.spawn_empty();
    ///
    ///     // Create another empty entity.
    ///     commands.spawn_empty()
    ///         // Add a new component bundle to the entity.
    ///         .insert((Strength(1), Agility(2)))
    ///         // Add a single component to the entity.
    ///         .insert(Label("hello world"));
    /// }
    /// # bevy_ecs::system::assert_is_system(example_system);
    /// ```
    ///
    /// # See also
    ///
    /// - [`spawn`](Self::spawn) to spawn an entity with components.
    /// - [`spawn_batch`](Self::spawn_batch) to spawn many entities
    ///   with the same combination of components.
    #[track_caller]
    pub fn spawn_empty(&mut self) -> EntityCommands<'_> {
        let entity = self.allocator.alloc();
        let caller = MaybeLocation::caller();
        self.queue(move |world: &mut World| {
            world.spawn_empty_at_with_caller(entity, caller).map(|_| ())
        });
        self.entity(entity)
    }

    /// Spawns a new [`Entity`] with the given components
    /// and returns the entity's corresponding [`EntityCommands`].
    ///
    /// To spawn many entities with the same combination of components,
    /// [`spawn_batch`](Self::spawn_batch) can be used for better performance.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component)]
    /// struct ComponentA(u32);
    /// #[derive(Component)]
    /// struct ComponentB(u32);
    ///
    /// #[derive(Bundle)]
    /// struct ExampleBundle {
    ///     a: ComponentA,
    ///     b: ComponentB,
    /// }
    ///
    /// fn example_system(mut commands: Commands) {
    ///     // Create a new entity with a single component.
    ///     commands.spawn(ComponentA(1));
    ///
    ///     // Create a new entity with two components using a "tuple bundle".
    ///     commands.spawn((ComponentA(2), ComponentB(1)));
    ///
    ///     // Create a new entity with a component bundle.
    ///     commands.spawn(ExampleBundle {
    ///         a: ComponentA(3),
    ///         b: ComponentB(2),
    ///     });
    /// }
    /// # bevy_ecs::system::assert_is_system(example_system);
    /// ```
    ///
    /// # See also
    ///
    /// - [`spawn_empty`](Self::spawn_empty) to spawn an entity without any components.
    /// - [`spawn_batch`](Self::spawn_batch) to spawn many entities
    ///   with the same combination of components.
    #[track_caller]
    pub fn spawn<T: Bundle>(&mut self, bundle: T) -> EntityCommands<'_> {
        let entity = self.allocator.alloc();
        let caller = MaybeLocation::caller();
        self.queue(move |world: &mut World| {
            move_as_ptr!(bundle);
            world
                .spawn_at_with_caller(entity, bundle, caller)
                .map(|_| ())
        });
        self.entity(entity)
    }

    /// Returns the [`EntityCommands`] for the given [`Entity`].
    ///
    /// This method does not guarantee that commands queued by the returned `EntityCommands`
    /// will be successful, since the entity could be despawned before they are executed.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Resource)]
    /// struct PlayerEntity {
    ///     entity: Entity
    /// }
    ///
    /// #[derive(Component)]
    /// struct Label(&'static str);
    ///
    /// fn example_system(mut commands: Commands, player: Res<PlayerEntity>) {
    ///     // Get the entity and add a component.
    ///     commands.entity(player.entity).insert(Label("hello world"));
    /// }
    /// # bevy_ecs::system::assert_is_system(example_system);
    /// ```
    ///
    /// # See also
    ///
    /// - [`get_entity`](Self::get_entity) for the fallible version.
    #[inline]
    #[track_caller]
    pub fn entity(&mut self, entity: Entity) -> EntityCommands<'_> {
        EntityCommands {
            entity,
            commands: self.reborrow(),
        }
    }

    /// Returns the [`EntityCommands`] for the requested [`Entity`] if it is valid.
    /// This method does not guarantee that commands queued by the returned `EntityCommands`
    /// will be successful, since the entity could be despawned before they are executed.
    /// This also does not error when the entity has not been spawned.
    /// For that behavior, see [`get_spawned_entity`](Self::get_spawned_entity),
    /// which should be preferred for accessing entities you expect to already be spawned, like those found from a query.
    /// For details on entity spawning vs validity, see [`entity`](crate::entity) module docs.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidEntityError`] if the requested entity does not exist.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Resource)]
    /// struct PlayerEntity {
    ///     entity: Entity
    /// }
    ///
    /// #[derive(Component)]
    /// struct Label(&'static str);
    ///
    /// fn example_system(mut commands: Commands, player: Res<PlayerEntity>) -> Result {
    ///     // Get the entity if it still exists and store the `EntityCommands`.
    ///     // If it doesn't exist, the `?` operator will propagate the returned error
    ///     // to the system, and the system will pass it to an error handler.
    ///     let mut entity_commands = commands.get_entity(player.entity)?;
    ///
    ///     // Add a component to the entity.
    ///     entity_commands.insert(Label("hello world"));
    ///
    ///     // Return from the system successfully.
    ///     Ok(())
    /// }
    /// # bevy_ecs::system::assert_is_system::<(), (), _>(example_system);
    /// ```
    ///
    /// # See also
    ///
    /// - [`entity`](Self::entity) for the infallible version.
    #[inline]
    #[track_caller]
    pub fn get_entity(&mut self, entity: Entity) -> Result<EntityCommands<'_>, InvalidEntityError> {
        let _location = self.entities.get(entity)?;
        Ok(EntityCommands {
            entity,
            commands: self.reborrow(),
        })
    }

    /// Returns the [`EntityCommands`] for the requested [`Entity`] if it spawned in the world *now*.
    /// Note that for entities that have not been spawned *yet*, like ones from [`spawn`](Self::spawn), this will error.
    /// If that is not desired, try [`get_entity`](Self::get_entity).
    /// This should be used over [`get_entity`](Self::get_entity) when you expect the entity to already be spawned in the world.
    /// If the entity is valid but not yet spawned, this will error that information, where [`get_entity`](Self::get_entity) would succeed, leading to potentially surprising results.
    /// For details on entity spawning vs validity, see [`entity`](crate::entity) module docs.
    ///
    /// This method does not guarantee that commands queued by the returned `EntityCommands`
    /// will be successful, since the entity could be despawned before they are executed.
    ///
    /// # Errors
    ///
    /// Returns [`EntityNotSpawnedError`] if the requested entity does not exist.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Resource)]
    /// struct PlayerEntity {
    ///     entity: Entity
    /// }
    ///
    /// #[derive(Component)]
    /// struct Label(&'static str);
    ///
    /// fn example_system(mut commands: Commands, player: Res<PlayerEntity>) -> Result {
    ///     // Get the entity if it still exists and store the `EntityCommands`.
    ///     // If it doesn't exist, the `?` operator will propagate the returned error
    ///     // to the system, and the system will pass it to an error handler.
    ///     let mut entity_commands = commands.get_spawned_entity(player.entity)?;
    ///
    ///     // Add a component to the entity.
    ///     entity_commands.insert(Label("hello world"));
    ///
    ///     // Return from the system successfully.
    ///     Ok(())
    /// }
    /// # bevy_ecs::system::assert_is_system::<(), (), _>(example_system);
    /// ```
    ///
    /// # See also
    ///
    /// - [`entity`](Self::entity) for the infallible version.
    #[inline]
    #[track_caller]
    pub fn get_spawned_entity(
        &mut self,
        entity: Entity,
    ) -> Result<EntityCommands<'_>, EntityNotSpawnedError> {
        let _location = self.entities.get_spawned(entity)?;
        Ok(EntityCommands {
            entity,
            commands: self.reborrow(),
        })
    }

    /// Spawns multiple entities with the same combination of components,
    /// based on a batch of [`Bundles`](Bundle).
    ///
    /// A batch can be any type that implements [`IntoIterator`] and contains bundles,
    /// such as a [`Vec<Bundle>`](alloc::vec::Vec) or an array `[Bundle; N]`.
    ///
    /// This method is equivalent to iterating the batch
    /// and calling [`spawn`](Self::spawn) for each bundle,
    /// but is faster by pre-allocating memory and having exclusive [`World`] access.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Score(u32);
    ///
    /// fn example_system(mut commands: Commands) {
    ///     commands.spawn_batch([
    ///         (Name::new("Alice"), Score(0)),
    ///         (Name::new("Bob"), Score(0)),
    ///     ]);
    /// }
    /// # bevy_ecs::system::assert_is_system(example_system);
    /// ```
    ///
    /// # See also
    ///
    /// - [`spawn`](Self::spawn) to spawn an entity with components.
    /// - [`spawn_empty`](Self::spawn_empty) to spawn an entity without components.
    #[track_caller]
    pub fn spawn_batch<I>(&mut self, batch: I)
    where
        I: IntoIterator + Send + Sync + 'static,
        I::Item: Bundle<Effect: NoBundleEffect>,
    {
        self.queue(command::spawn_batch(batch));
    }

    /// Pushes a generic [`Command`] to the command queue.
    ///
    /// If the [`Command`] returns a [`Result`],
    /// it will be handled using the [default error handler](crate::error::DefaultErrorHandler).
    ///
    /// To use a custom error handler, see [`Commands::queue_handled`].
    ///
    /// The command can be:
    /// - A custom struct that implements [`Command`].
    /// - A closure or function that matches one of the following signatures:
    ///   - [`(&mut World)`](World)
    /// - A built-in command from the [`command`] module.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Resource, Default)]
    /// struct Counter(u64);
    ///
    /// struct AddToCounter(String);
    ///
    /// impl Command<Result> for AddToCounter {
    ///     fn apply(self, world: &mut World) -> Result {
    ///         let mut counter = world.get_resource_or_insert_with(Counter::default);
    ///         let amount: u64 = self.0.parse()?;
    ///         counter.0 += amount;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// fn add_three_to_counter_system(mut commands: Commands) {
    ///     commands.queue(AddToCounter("3".to_string()));
    /// }
    ///
    /// fn add_twenty_five_to_counter_system(mut commands: Commands) {
    ///     commands.queue(|world: &mut World| {
    ///         let mut counter = world.get_resource_or_insert_with(Counter::default);
    ///         counter.0 += 25;
    ///     });
    /// }
    /// # bevy_ecs::system::assert_is_system(add_three_to_counter_system);
    /// # bevy_ecs::system::assert_is_system(add_twenty_five_to_counter_system);
    /// ```
    pub fn queue<C: Command<T> + HandleError<T>, T>(&mut self, command: C) {
        self.queue_internal(command.handle_error());
    }

    /// Pushes a generic [`Command`] to the command queue.
    ///
    /// If the [`Command`] returns a [`Result`],
    /// the given `error_handler` will be used to handle error cases.
    ///
    /// To implicitly use the default error handler, see [`Commands::queue`].
    ///
    /// The command can be:
    /// - A custom struct that implements [`Command`].
    /// - A closure or function that matches one of the following signatures:
    ///   - [`(&mut World)`](World)
    ///   - [`(&mut World)`](World) `->` [`Result`]
    /// - A built-in command from the [`command`] module.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// use bevy_ecs::error::warn;
    ///
    /// #[derive(Resource, Default)]
    /// struct Counter(u64);
    ///
    /// struct AddToCounter(String);
    ///
    /// impl Command<Result> for AddToCounter {
    ///     fn apply(self, world: &mut World) -> Result {
    ///         let mut counter = world.get_resource_or_insert_with(Counter::default);
    ///         let amount: u64 = self.0.parse()?;
    ///         counter.0 += amount;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// fn add_three_to_counter_system(mut commands: Commands) {
    ///     commands.queue_handled(AddToCounter("3".to_string()), warn);
    /// }
    ///
    /// fn add_twenty_five_to_counter_system(mut commands: Commands) {
    ///     commands.queue(|world: &mut World| {
    ///         let mut counter = world.get_resource_or_insert_with(Counter::default);
    ///         counter.0 += 25;
    ///     });
    /// }
    /// # bevy_ecs::system::assert_is_system(add_three_to_counter_system);
    /// # bevy_ecs::system::assert_is_system(add_twenty_five_to_counter_system);
    /// ```
    pub fn queue_handled<C: Command<T> + HandleError<T>, T>(
        &mut self,
        command: C,
        error_handler: fn(BevyError, ErrorContext),
    ) {
        self.queue_internal(command.handle_error_with(error_handler));
    }

    /// Pushes a generic [`Command`] to the queue like [`Commands::queue_handled`], but instead silently ignores any errors.
    pub fn queue_silenced<C: Command<T> + HandleError<T>, T>(&mut self, command: C) {
        self.queue_internal(command.ignore_error());
    }

    fn queue_internal(&mut self, command: impl Command) {
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

    /// Adds a series of [`Bundles`](Bundle) to each [`Entity`] they are paired with,
    /// based on a batch of `(Entity, Bundle)` pairs.
    ///
    /// A batch can be any type that implements [`IntoIterator`]
    /// and contains `(Entity, Bundle)` tuples,
    /// such as a [`Vec<(Entity, Bundle)>`](alloc::vec::Vec)
    /// or an array `[(Entity, Bundle); N]`.
    ///
    /// This will overwrite any pre-existing components shared by the [`Bundle`] type.
    /// Use [`Commands::insert_batch_if_new`] to keep the pre-existing components instead.
    ///
    /// This method is equivalent to iterating the batch
    /// and calling [`insert`](EntityCommands::insert) for each pair,
    /// but is faster by caching data that is shared between entities.
    ///
    /// # Fallible
    ///
    /// This command will fail if any of the given entities do not exist.
    ///
    /// It will internally return a [`TryInsertBatchError`](crate::world::error::TryInsertBatchError),
    /// which will be handled by the [default error handler](crate::error::DefaultErrorHandler).
    #[track_caller]
    pub fn insert_batch<I, B>(&mut self, batch: I)
    where
        I: IntoIterator<Item = (Entity, B)> + Send + Sync + 'static,
        B: Bundle<Effect: NoBundleEffect>,
    {
        self.queue(command::insert_batch(batch, InsertMode::Replace));
    }

    /// Adds a series of [`Bundles`](Bundle) to each [`Entity`] they are paired with,
    /// based on a batch of `(Entity, Bundle)` pairs.
    ///
    /// A batch can be any type that implements [`IntoIterator`]
    /// and contains `(Entity, Bundle)` tuples,
    /// such as a [`Vec<(Entity, Bundle)>`](alloc::vec::Vec)
    /// or an array `[(Entity, Bundle); N]`.
    ///
    /// This will keep any pre-existing components shared by the [`Bundle`] type
    /// and discard the new values.
    /// Use [`Commands::insert_batch`] to overwrite the pre-existing components instead.
    ///
    /// This method is equivalent to iterating the batch
    /// and calling [`insert_if_new`](EntityCommands::insert_if_new) for each pair,
    /// but is faster by caching data that is shared between entities.
    ///
    /// # Fallible
    ///
    /// This command will fail if any of the given entities do not exist.
    ///
    /// It will internally return a [`TryInsertBatchError`](crate::world::error::TryInsertBatchError),
    /// which will be handled by the [default error handler](crate::error::DefaultErrorHandler).
    #[track_caller]
    pub fn insert_batch_if_new<I, B>(&mut self, batch: I)
    where
        I: IntoIterator<Item = (Entity, B)> + Send + Sync + 'static,
        B: Bundle<Effect: NoBundleEffect>,
    {
        self.queue(command::insert_batch(batch, InsertMode::Keep));
    }

    /// Adds a series of [`Bundles`](Bundle) to each [`Entity`] they are paired with,
    /// based on a batch of `(Entity, Bundle)` pairs.
    ///
    /// A batch can be any type that implements [`IntoIterator`]
    /// and contains `(Entity, Bundle)` tuples,
    /// such as a [`Vec<(Entity, Bundle)>`](alloc::vec::Vec)
    /// or an array `[(Entity, Bundle); N]`.
    ///
    /// This will overwrite any pre-existing components shared by the [`Bundle`] type.
    /// Use [`Commands::try_insert_batch_if_new`] to keep the pre-existing components instead.
    ///
    /// This method is equivalent to iterating the batch
    /// and calling [`insert`](EntityCommands::insert) for each pair,
    /// but is faster by caching data that is shared between entities.
    ///
    /// # Fallible
    ///
    /// This command will fail if any of the given entities do not exist.
    ///
    /// It will internally return a [`TryInsertBatchError`](crate::world::error::TryInsertBatchError),
    /// which will be handled by [logging the error at the `warn` level](warn).
    #[track_caller]
    pub fn try_insert_batch<I, B>(&mut self, batch: I)
    where
        I: IntoIterator<Item = (Entity, B)> + Send + Sync + 'static,
        B: Bundle<Effect: NoBundleEffect>,
    {
        self.queue(command::insert_batch(batch, InsertMode::Replace).handle_error_with(warn));
    }

    /// Adds a series of [`Bundles`](Bundle) to each [`Entity`] they are paired with,
    /// based on a batch of `(Entity, Bundle)` pairs.
    ///
    /// A batch can be any type that implements [`IntoIterator`]
    /// and contains `(Entity, Bundle)` tuples,
    /// such as a [`Vec<(Entity, Bundle)>`](alloc::vec::Vec)
    /// or an array `[(Entity, Bundle); N]`.
    ///
    /// This will keep any pre-existing components shared by the [`Bundle`] type
    /// and discard the new values.
    /// Use [`Commands::try_insert_batch`] to overwrite the pre-existing components instead.
    ///
    /// This method is equivalent to iterating the batch
    /// and calling [`insert_if_new`](EntityCommands::insert_if_new) for each pair,
    /// but is faster by caching data that is shared between entities.
    ///
    /// # Fallible
    ///
    /// This command will fail if any of the given entities do not exist.
    ///
    /// It will internally return a [`TryInsertBatchError`](crate::world::error::TryInsertBatchError),
    /// which will be handled by [logging the error at the `warn` level](warn).
    #[track_caller]
    pub fn try_insert_batch_if_new<I, B>(&mut self, batch: I)
    where
        I: IntoIterator<Item = (Entity, B)> + Send + Sync + 'static,
        B: Bundle<Effect: NoBundleEffect>,
    {
        self.queue(command::insert_batch(batch, InsertMode::Keep).handle_error_with(warn));
    }

    /// Inserts a [`Resource`] into the [`World`] with an inferred value.
    ///
    /// The inferred value is determined by the [`FromWorld`] trait of the resource.
    /// Note that any resource with the [`Default`] trait automatically implements [`FromWorld`],
    /// and those default values will be used.
    ///
    /// If the resource already exists when the command is applied, nothing happens.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Resource, Default)]
    /// struct Scoreboard {
    ///     current_score: u32,
    ///     high_score: u32,
    /// }
    ///
    /// fn initialize_scoreboard(mut commands: Commands) {
    ///     commands.init_resource::<Scoreboard>();
    /// }
    /// # bevy_ecs::system::assert_is_system(initialize_scoreboard);
    /// ```
    #[track_caller]
    pub fn init_resource<R: Resource + FromWorld>(&mut self) {
        self.queue(command::init_resource::<R>());
    }

    /// Inserts a [`Resource`] into the [`World`] with a specific value.
    ///
    /// This will overwrite any previous value of the same resource type.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Resource)]
    /// struct Scoreboard {
    ///     current_score: u32,
    ///     high_score: u32,
    /// }
    ///
    /// fn system(mut commands: Commands) {
    ///     commands.insert_resource(Scoreboard {
    ///         current_score: 0,
    ///         high_score: 0,
    ///     });
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[track_caller]
    pub fn insert_resource<R: Resource>(&mut self, resource: R) {
        self.queue(command::insert_resource(resource));
    }

    /// Removes a [`Resource`] from the [`World`].
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Resource)]
    /// struct Scoreboard {
    ///     current_score: u32,
    ///     high_score: u32,
    /// }
    ///
    /// fn system(mut commands: Commands) {
    ///     commands.remove_resource::<Scoreboard>();
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn remove_resource<R: Resource>(&mut self) {
        self.queue(command::remove_resource::<R>());
    }

    /// Runs the system corresponding to the given [`SystemId`].
    /// Before running a system, it must first be registered via
    /// [`Commands::register_system`] or [`World::register_system`].
    ///
    /// The system is run in an exclusive and single-threaded way.
    /// Running slow systems can become a bottleneck.
    ///
    /// There is no way to get the output of a system when run as a command, because the
    /// execution of the system happens later. To get the output of a system, use
    /// [`World::run_system`] or [`World::run_system_with`] instead of running the system as a command.
    ///
    /// # Fallible
    ///
    /// This command will fail if the given [`SystemId`]
    /// does not correspond to a [`System`](crate::system::System).
    ///
    /// It will internally return a [`RegisteredSystemError`](crate::system::system_registry::RegisteredSystemError),
    /// which will be handled by [logging the error at the `warn` level](warn).
    pub fn run_system(&mut self, id: SystemId) {
        self.queue(command::run_system(id).handle_error_with(warn));
    }

    /// Runs the system corresponding to the given [`SystemId`] with input.
    /// Before running a system, it must first be registered via
    /// [`Commands::register_system`] or [`World::register_system`].
    ///
    /// The system is run in an exclusive and single-threaded way.
    /// Running slow systems can become a bottleneck.
    ///
    /// There is no way to get the output of a system when run as a command, because the
    /// execution of the system happens later. To get the output of a system, use
    /// [`World::run_system`] or [`World::run_system_with`] instead of running the system as a command.
    ///
    /// # Fallible
    ///
    /// This command will fail if the given [`SystemId`]
    /// does not correspond to a [`System`](crate::system::System).
    ///
    /// It will internally return a [`RegisteredSystemError`](crate::system::system_registry::RegisteredSystemError),
    /// which will be handled by [logging the error at the `warn` level](warn).
    pub fn run_system_with<I>(&mut self, id: SystemId<I>, input: I::Inner<'static>)
    where
        I: SystemInput<Inner<'static>: Send> + 'static,
    {
        self.queue(command::run_system_with(id, input).handle_error_with(warn));
    }

    /// Registers a system and returns its [`SystemId`] so it can later be called by
    /// [`Commands::run_system`] or [`World::run_system`].
    ///
    /// This is different from adding systems to a [`Schedule`](crate::schedule::Schedule),
    /// because the [`SystemId`] that is returned can be used anywhere in the [`World`] to run the associated system.
    ///
    /// Using a [`Schedule`](crate::schedule::Schedule) is still preferred for most cases
    /// due to its better performance and ability to run non-conflicting systems simultaneously.
    ///
    /// # Note
    ///
    /// If the same system is registered more than once,
    /// each registration will be considered a different system,
    /// and they will each be given their own [`SystemId`].
    ///
    /// If you want to avoid registering the same system multiple times,
    /// consider using [`Commands::run_system_cached`] or storing the [`SystemId`]
    /// in a [`Local`](crate::system::Local).
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, world::CommandQueue, system::SystemId};
    /// #[derive(Resource)]
    /// struct Counter(i32);
    ///
    /// fn register_system(
    ///     mut commands: Commands,
    ///     mut local_system: Local<Option<SystemId>>,
    /// ) {
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
        let system = RegisteredSystem::<I, O>::new(Box::new(IntoSystem::into_system(system)));
        self.entity(entity).insert(system);
        SystemId::from_entity(entity)
    }

    /// Removes a system previously registered with [`Commands::register_system`]
    /// or [`World::register_system`].
    ///
    /// After removing a system, the [`SystemId`] becomes invalid
    /// and attempting to use it afterwards will result in an error.
    /// Re-adding the removed system will register it with a new `SystemId`.
    ///
    /// # Fallible
    ///
    /// This command will fail if the given [`SystemId`]
    /// does not correspond to a [`System`](crate::system::System).
    ///
    /// It will internally return a [`RegisteredSystemError`](crate::system::system_registry::RegisteredSystemError),
    /// which will be handled by [logging the error at the `warn` level](warn).
    pub fn unregister_system<I, O>(&mut self, system_id: SystemId<I, O>)
    where
        I: SystemInput + Send + 'static,
        O: Send + 'static,
    {
        self.queue(command::unregister_system(system_id).handle_error_with(warn));
    }

    /// Removes a system previously registered with one of the following:
    /// - [`Commands::run_system_cached`]
    /// - [`World::run_system_cached`]
    /// - [`World::register_system_cached`]
    ///
    /// # Fallible
    ///
    /// This command will fail if the given system
    /// is not currently cached in a [`CachedSystemId`](crate::system::CachedSystemId) resource.
    ///
    /// It will internally return a [`RegisteredSystemError`](crate::system::system_registry::RegisteredSystemError),
    /// which will be handled by [logging the error at the `warn` level](warn).
    pub fn unregister_system_cached<I, O, M, S>(&mut self, system: S)
    where
        I: SystemInput + Send + 'static,
        O: 'static,
        M: 'static,
        S: IntoSystem<I, O, M> + Send + 'static,
    {
        self.queue(command::unregister_system_cached(system).handle_error_with(warn));
    }

    /// Runs a cached system, registering it if necessary.
    ///
    /// Unlike [`Commands::run_system`], this method does not require manual registration.
    ///
    /// The first time this method is called for a particular system,
    /// it will register the system and store its [`SystemId`] in a
    /// [`CachedSystemId`](crate::system::CachedSystemId) resource for later.
    ///
    /// If you would rather manage the [`SystemId`] yourself,
    /// or register multiple copies of the same system,
    /// use [`Commands::register_system`] instead.
    ///
    /// # Limitations
    ///
    /// This method only accepts ZST (zero-sized) systems to guarantee that any two systems of
    /// the same type must be equal. This means that closures that capture the environment, and
    /// function pointers, are not accepted.
    ///
    /// If you want to access values from the environment within a system,
    /// consider passing them in as inputs via [`Commands::run_system_cached_with`].
    ///
    /// If that's not an option, consider [`Commands::register_system`] instead.
    pub fn run_system_cached<M, S>(&mut self, system: S)
    where
        M: 'static,
        S: IntoSystem<(), (), M> + Send + 'static,
    {
        self.queue(command::run_system_cached(system).handle_error_with(warn));
    }

    /// Runs a cached system with an input, registering it if necessary.
    ///
    /// Unlike [`Commands::run_system_with`], this method does not require manual registration.
    ///
    /// The first time this method is called for a particular system,
    /// it will register the system and store its [`SystemId`] in a
    /// [`CachedSystemId`](crate::system::CachedSystemId) resource for later.
    ///
    /// If you would rather manage the [`SystemId`] yourself,
    /// or register multiple copies of the same system,
    /// use [`Commands::register_system`] instead.
    ///
    /// # Limitations
    ///
    /// This method only accepts ZST (zero-sized) systems to guarantee that any two systems of
    /// the same type must be equal. This means that closures that capture the environment, and
    /// function pointers, are not accepted.
    ///
    /// If you want to access values from the environment within a system,
    /// consider passing them in as inputs.
    ///
    /// If that's not an option, consider [`Commands::register_system`] instead.
    pub fn run_system_cached_with<I, M, S>(&mut self, system: S, input: I::Inner<'static>)
    where
        I: SystemInput<Inner<'static>: Send> + Send + 'static,
        M: 'static,
        S: IntoSystem<I, (), M> + Send + 'static,
    {
        self.queue(command::run_system_cached_with(system, input).handle_error_with(warn));
    }

    /// Triggers the given [`Event`], which will run any [`Observer`]s watching for it.
    ///
    /// [`Observer`]: crate::observer::Observer
    #[track_caller]
    pub fn trigger<'a>(&mut self, event: impl Event<Trigger<'a>: Default>) {
        self.queue(command::trigger(event));
    }

    /// Triggers the given [`Event`] using the given [`Trigger`], which will run any [`Observer`]s watching for it.
    ///
    /// [`Trigger`]: crate::event::Trigger
    /// [`Observer`]: crate::observer::Observer
    #[track_caller]
    pub fn trigger_with<E: Event<Trigger<'static>: Send + Sync>>(
        &mut self,
        event: E,
        trigger: E::Trigger<'static>,
    ) {
        self.queue(command::trigger_with(event, trigger));
    }

    /// Spawns an [`Observer`](crate::observer::Observer) and returns the [`EntityCommands`] associated
    /// with the entity that stores the observer.
    ///
    /// `observer` can be any system whose first parameter is [`On`].
    ///
    /// **Calling [`observe`](EntityCommands::observe) on the returned
    /// [`EntityCommands`] will observe the observer itself, which you very
    /// likely do not want.**
    ///
    /// # Panics
    ///
    /// Panics if the given system is an exclusive system.
    ///
    /// [`On`]: crate::observer::On
    pub fn add_observer<M>(&mut self, observer: impl IntoObserver<M>) -> EntityCommands<'_> {
        self.spawn(observer.into_observer())
    }

    /// Writes an arbitrary [`Message`].
    ///
    /// This is a convenience method for writing messages
    /// without requiring a [`MessageWriter`](crate::message::MessageWriter).
    ///
    /// # Performance
    ///
    /// Since this is a command, exclusive world access is used, which means that it will not profit from
    /// system-level parallelism on supported platforms.
    ///
    /// If these messages are performance-critical or very frequently sent,
    /// consider using a [`MessageWriter`](crate::message::MessageWriter) instead.
    #[track_caller]
    pub fn write_message<M: Message>(&mut self, message: M) -> &mut Self {
        self.queue(command::write_message(message));
        self
    }

    /// Runs the schedule corresponding to the given [`ScheduleLabel`].
    ///
    /// Calls [`World::try_run_schedule`](World::try_run_schedule).
    ///
    /// # Fallible
    ///
    /// This command will fail if the given [`ScheduleLabel`]
    /// does not correspond to a [`Schedule`](crate::schedule::Schedule).
    ///
    /// It will internally return a [`TryRunScheduleError`](crate::world::error::TryRunScheduleError),
    /// which will be handled by [logging the error at the `warn` level](warn).
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ecs::schedule::ScheduleLabel;
    /// # #[derive(Default, Resource)]
    /// # struct Counter(u32);
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
        self.queue(command::run_schedule(label).handle_error_with(warn));
    }
}

/// A list of commands that will be run to modify an [`Entity`].
///
/// # Note
///
/// Most [`Commands`] (and thereby [`EntityCommands`]) are deferred:
/// when you call the command, if it requires mutable access to the [`World`]
/// (that is, if it removes, adds, or changes something), it's not executed immediately.
///
/// Instead, the command is added to a "command queue."
/// The command queue is applied later
/// when the [`ApplyDeferred`](crate::schedule::ApplyDeferred) system runs.
/// Commands are executed one-by-one so that
/// each command can have exclusive access to the `World`.
///
/// # Fallible
///
/// Due to their deferred nature, an entity you're trying to change with an [`EntityCommand`]
/// can be despawned by the time the command is executed.
///
/// All deferred entity commands will check whether the entity exists at the time of execution
/// and will return an error if it doesn't.
///
/// # Error handling
///
/// An [`EntityCommand`] can return a [`Result`](crate::error::Result),
/// which will be passed to an [error handler](crate::error) if the `Result` is an error.
///
/// The default error handler panics. It can be configured via
/// the [`DefaultErrorHandler`](crate::error::DefaultErrorHandler) resource.
///
/// Alternatively, you can customize the error handler for a specific command
/// by calling [`EntityCommands::queue_handled`].
///
/// The [`error`](crate::error) module provides some simple error handlers for convenience.
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
    ///
    /// This is useful if you have `&mut EntityCommands` but you need `EntityCommands`.
    pub fn reborrow(&mut self) -> EntityCommands<'_> {
        EntityCommands {
            entity: self.entity,
            commands: self.commands.reborrow(),
        }
    }

    /// Get an [`EntityEntryCommands`] for the [`Component`] `T`,
    /// allowing you to modify it or insert it if it isn't already present.
    ///
    /// See also [`insert_if_new`](Self::insert_if_new),
    /// which lets you insert a [`Bundle`] without overwriting it.
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
    ///
    /// #[derive(Component, Default)]
    /// struct Mana {
    ///     max: u32,
    ///     current: u32,
    /// }
    ///
    /// fn level_up_system(mut commands: Commands, player: Res<PlayerEntity>) {
    ///     // If a component already exists then modify it, otherwise insert a default value
    ///     commands
    ///         .entity(player.entity)
    ///         .entry::<Level>()
    ///         .and_modify(|mut lvl| lvl.0 += 1)
    ///         .or_insert(Level(0));
    ///
    ///     // Add a default value if none exists, and then modify the existing or new value
    ///     commands
    ///         .entity(player.entity)
    ///         .entry::<Mana>()
    ///         .or_default()
    ///         .and_modify(|mut mana| {
    ///             mana.max += 10;
    ///             mana.current = mana.max;
    ///     });
    /// }
    ///
    /// # bevy_ecs::system::assert_is_system(level_up_system);
    /// ```
    pub fn entry<T: Component>(&mut self) -> EntityEntryCommands<'_, T> {
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
        self.queue(entity_command::insert(bundle, InsertMode::Replace))
    }

    /// Adds a [`Bundle`] of components to the entity if the predicate returns true.
    ///
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
            self.insert(bundle)
        } else {
            self
        }
    }

    /// Adds a [`Bundle`] of components to the entity without overwriting.
    ///
    /// This is the same as [`EntityCommands::insert`], but in case of duplicate
    /// components will leave the old values instead of replacing them with new ones.
    ///
    /// See also [`entry`](Self::entry), which lets you modify a [`Component`] if it's present,
    /// as well as initialize it with a default value.
    #[track_caller]
    pub fn insert_if_new(&mut self, bundle: impl Bundle) -> &mut Self {
        self.queue(entity_command::insert(bundle, InsertMode::Keep))
    }

    /// Adds a [`Bundle`] of components to the entity without overwriting if the
    /// predicate returns true.
    ///
    /// This is the same as [`EntityCommands::insert_if`], but in case of duplicate
    /// components will leave the old values instead of replacing them with new ones.
    #[track_caller]
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

    /// Adds a dynamic [`Component`] to the entity.
    ///
    /// This will overwrite any previous value(s) of the same component type.
    ///
    /// You should prefer to use the typed API [`EntityCommands::insert`] where possible.
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
        self.queue(
            // SAFETY:
            // - `ComponentId` safety is ensured by the caller.
            // - `T` safety is ensured by the caller.
            unsafe { entity_command::insert_by_id(component_id, value, InsertMode::Replace) },
        )
    }

    /// Adds a dynamic [`Component`] to the entity.
    ///
    /// This will overwrite any previous value(s) of the same component type.
    ///
    /// You should prefer to use the typed API [`EntityCommands::try_insert`] where possible.
    ///
    /// # Note
    ///
    /// If the entity does not exist when this command is executed,
    /// the resulting error will be ignored.
    ///
    /// # Safety
    ///
    /// - [`ComponentId`] must be from the same world as `self`.
    /// - `T` must have the same layout as the one passed during `component_id` creation.
    #[track_caller]
    pub unsafe fn try_insert_by_id<T: Send + 'static>(
        &mut self,
        component_id: ComponentId,
        value: T,
    ) -> &mut Self {
        self.queue_silenced(
            // SAFETY:
            // - `ComponentId` safety is ensured by the caller.
            // - `T` safety is ensured by the caller.
            unsafe { entity_command::insert_by_id(component_id, value, InsertMode::Replace) },
        )
    }

    /// Adds a [`Bundle`] of components to the entity.
    ///
    /// This will overwrite any previous value(s) of the same component type.
    ///
    /// # Note
    ///
    /// If the entity does not exist when this command is executed,
    /// the resulting error will be ignored.
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
    ///     commands.entity(player.entity)
    ///         // You can insert individual components:
    ///         .try_insert(Defense(10))
    ///         // You can also insert tuples of components:
    ///         .try_insert(CombatBundle {
    ///             health: Health(100),
    ///             strength: Strength(40),
    ///         });
    ///
    ///     // Suppose this occurs in a parallel adjacent system or process.
    ///     commands.entity(player.entity).despawn();
    ///
    ///     // This will not panic nor will it add the component.
    ///     commands.entity(player.entity).try_insert(Defense(5));
    /// }
    /// # bevy_ecs::system::assert_is_system(add_combat_stats_system);
    /// ```
    #[track_caller]
    pub fn try_insert(&mut self, bundle: impl Bundle) -> &mut Self {
        self.queue_silenced(entity_command::insert(bundle, InsertMode::Replace))
    }

    /// Adds a [`Bundle`] of components to the entity if the predicate returns true.
    ///
    /// This is useful for chaining method calls.
    ///
    /// # Note
    ///
    /// If the entity does not exist when this command is executed,
    /// the resulting error will be ignored.
    #[track_caller]
    pub fn try_insert_if<F>(&mut self, bundle: impl Bundle, condition: F) -> &mut Self
    where
        F: FnOnce() -> bool,
    {
        if condition() {
            self.try_insert(bundle)
        } else {
            self
        }
    }

    /// Adds a [`Bundle`] of components to the entity without overwriting if the
    /// predicate returns true.
    ///
    /// This is the same as [`EntityCommands::try_insert_if`], but in case of duplicate
    /// components will leave the old values instead of replacing them with new ones.
    ///
    /// # Note
    ///
    /// If the entity does not exist when this command is executed,
    /// the resulting error will be ignored.
    #[track_caller]
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

    /// Adds a [`Bundle`] of components to the entity without overwriting.
    ///
    /// This is the same as [`EntityCommands::try_insert`], but in case of duplicate
    /// components will leave the old values instead of replacing them with new ones.
    ///
    /// # Note
    ///
    /// If the entity does not exist when this command is executed,
    /// the resulting error will be ignored.
    #[track_caller]
    pub fn try_insert_if_new(&mut self, bundle: impl Bundle) -> &mut Self {
        self.queue_silenced(entity_command::insert(bundle, InsertMode::Keep))
    }

    /// Removes a [`Bundle`] of components from the entity.
    ///
    /// This will remove all components that intersect with the provided bundle;
    /// the entity does not need to have all the components in the bundle.
    ///
    /// This will emit a warning if the entity does not exist.
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
    /// fn remove_combat_stats_system(mut commands: Commands, player: Res<PlayerEntity>) {
    ///     commands
    ///         .entity(player.entity)
    ///         // You can remove individual components:
    ///         .remove::<Defense>()
    ///         // You can also remove pre-defined bundles of components:
    ///         .remove::<CombatBundle>()
    ///         // You can also remove tuples of components and bundles.
    ///         // This is equivalent to the calls above:
    ///         .remove::<(Defense, CombatBundle)>();
    /// }
    /// # bevy_ecs::system::assert_is_system(remove_combat_stats_system);
    /// ```
    #[track_caller]
    pub fn remove<B: Bundle>(&mut self) -> &mut Self {
        self.queue_handled(entity_command::remove::<B>(), warn)
    }

    /// Removes a [`Bundle`] of components from the entity if the predicate returns true.
    ///
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
    ///         .remove_if::<(Defense, CombatBundle)>(|| !player.is_spectator());
    /// }
    /// # bevy_ecs::system::assert_is_system(remove_combat_stats_system);
    /// ```
    #[track_caller]
    pub fn remove_if<B: Bundle>(&mut self, condition: impl FnOnce() -> bool) -> &mut Self {
        if condition() {
            self.remove::<B>()
        } else {
            self
        }
    }

    /// Removes a [`Bundle`] of components from the entity if the predicate returns true.
    ///
    /// This is useful for chaining method calls.
    ///
    /// # Note
    ///
    /// If the entity does not exist when this command is executed,
    /// the resulting error will be ignored.
    #[track_caller]
    pub fn try_remove_if<B: Bundle>(&mut self, condition: impl FnOnce() -> bool) -> &mut Self {
        if condition() {
            self.try_remove::<B>()
        } else {
            self
        }
    }

    /// Removes a [`Bundle`] of components from the entity.
    ///
    /// This will remove all components that intersect with the provided bundle;
    /// the entity does not need to have all the components in the bundle.
    ///
    /// Unlike [`Self::remove`],
    /// this will not emit a warning if the entity does not exist.
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
    /// fn remove_combat_stats_system(mut commands: Commands, player: Res<PlayerEntity>) {
    ///     commands
    ///         .entity(player.entity)
    ///         // You can remove individual components:
    ///         .try_remove::<Defense>()
    ///         // You can also remove pre-defined bundles of components:
    ///         .try_remove::<CombatBundle>()
    ///         // You can also remove tuples of components and bundles.
    ///         // This is equivalent to the calls above:
    ///         .try_remove::<(Defense, CombatBundle)>();
    /// }
    /// # bevy_ecs::system::assert_is_system(remove_combat_stats_system);
    /// ```
    pub fn try_remove<B: Bundle>(&mut self) -> &mut Self {
        self.queue_silenced(entity_command::remove::<B>())
    }

    /// Removes a [`Bundle`] of components from the entity,
    /// and also removes any components required by the components in the bundle.
    ///
    /// This will remove all components that intersect with the provided bundle;
    /// the entity does not need to have all the components in the bundle.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource)]
    /// # struct PlayerEntity { entity: Entity }
    /// #
    /// #[derive(Component)]
    /// #[require(B)]
    /// struct A;
    /// #[derive(Component, Default)]
    /// struct B;
    ///
    /// fn remove_with_requires_system(mut commands: Commands, player: Res<PlayerEntity>) {
    ///     commands
    ///         .entity(player.entity)
    ///         // Removes both A and B from the entity, because B is required by A.
    ///         .remove_with_requires::<A>();
    /// }
    /// # bevy_ecs::system::assert_is_system(remove_with_requires_system);
    /// ```
    #[track_caller]
    pub fn remove_with_requires<B: Bundle>(&mut self) -> &mut Self {
        self.queue(entity_command::remove_with_requires::<B>())
    }

    /// Removes a dynamic [`Component`] from the entity if it exists.
    ///
    /// # Panics
    ///
    /// Panics if the provided [`ComponentId`] does not exist in the [`World`].
    #[track_caller]
    pub fn remove_by_id(&mut self, component_id: ComponentId) -> &mut Self {
        self.queue(entity_command::remove_by_id(component_id))
    }

    /// Removes all components associated with the entity.
    #[track_caller]
    pub fn clear(&mut self) -> &mut Self {
        self.queue(entity_command::clear())
    }

    /// Despawns the entity.
    ///
    /// This will emit a warning if the entity does not exist.
    ///
    /// # Note
    ///
    /// This will also despawn the entities in any [`RelationshipTarget`](crate::relationship::RelationshipTarget)
    /// that is configured to despawn descendants.
    ///
    /// For example, this will recursively despawn [`Children`](crate::hierarchy::Children).
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource)]
    /// # struct CharacterToRemove { entity: Entity }
    /// #
    /// fn remove_character_system(
    ///     mut commands: Commands,
    ///     character_to_remove: Res<CharacterToRemove>
    /// ) {
    ///     commands.entity(character_to_remove.entity).despawn();
    /// }
    /// # bevy_ecs::system::assert_is_system(remove_character_system);
    /// ```
    #[track_caller]
    pub fn despawn(&mut self) {
        self.queue_handled(entity_command::despawn(), warn);
    }

    /// Despawns the entity.
    ///
    /// Unlike [`Self::despawn`],
    /// this will not emit a warning if the entity does not exist.
    ///
    /// # Note
    ///
    /// This will also despawn the entities in any [`RelationshipTarget`](crate::relationship::RelationshipTarget)
    /// that is configured to despawn descendants.
    ///
    /// For example, this will recursively despawn [`Children`](crate::hierarchy::Children).
    pub fn try_despawn(&mut self) {
        self.queue_silenced(entity_command::despawn());
    }

    /// Pushes an [`EntityCommand`] to the queue,
    /// which will get executed for the current [`Entity`].
    ///
    /// The [default error handler](crate::error::DefaultErrorHandler)
    /// will be used to handle error cases.
    /// Every [`EntityCommand`] checks whether the entity exists at the time of execution
    /// and returns an error if it does not.
    ///
    /// To use a custom error handler, see [`EntityCommands::queue_handled`].
    ///
    /// The command can be:
    /// - A custom struct that implements [`EntityCommand`].
    /// - A closure or function that matches the following signature:
    ///   - [`(EntityWorldMut)`](EntityWorldMut)
    ///   - [`(EntityWorldMut)`](EntityWorldMut) `->` [`Result`]
    /// - A built-in command from the [`entity_command`] module.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # fn my_system(mut commands: Commands) {
    /// commands
    ///     .spawn_empty()
    ///     // Closures with this signature implement `EntityCommand`.
    ///     .queue(|entity: EntityWorldMut| {
    ///         println!("Executed an EntityCommand for {}", entity.id());
    ///     });
    /// # }
    /// # bevy_ecs::system::assert_is_system(my_system);
    /// ```
    pub fn queue<C: EntityCommand<T> + CommandWithEntity<M>, T, M>(
        &mut self,
        command: C,
    ) -> &mut Self {
        self.commands.queue(command.with_entity(self.entity));
        self
    }

    /// Pushes an [`EntityCommand`] to the queue,
    /// which will get executed for the current [`Entity`].
    ///
    /// The given `error_handler` will be used to handle error cases.
    /// Every [`EntityCommand`] checks whether the entity exists at the time of execution
    /// and returns an error if it does not.
    ///
    /// To implicitly use the default error handler, see [`EntityCommands::queue`].
    ///
    /// The command can be:
    /// - A custom struct that implements [`EntityCommand`].
    /// - A closure or function that matches the following signature:
    ///   - [`(EntityWorldMut)`](EntityWorldMut)
    ///   - [`(EntityWorldMut)`](EntityWorldMut) `->` [`Result`]
    /// - A built-in command from the [`entity_command`] module.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # fn my_system(mut commands: Commands) {
    /// use bevy_ecs::error::warn;
    ///
    /// commands
    ///     .spawn_empty()
    ///     // Closures with this signature implement `EntityCommand`.
    ///     .queue_handled(
    ///         |entity: EntityWorldMut| -> Result {
    ///             let value: usize = "100".parse()?;
    ///             println!("Successfully parsed the value {} for entity {}", value, entity.id());
    ///             Ok(())
    ///         },
    ///         warn
    ///     );
    /// # }
    /// # bevy_ecs::system::assert_is_system(my_system);
    /// ```
    pub fn queue_handled<C: EntityCommand<T> + CommandWithEntity<M>, T, M>(
        &mut self,
        command: C,
        error_handler: fn(BevyError, ErrorContext),
    ) -> &mut Self {
        self.commands
            .queue_handled(command.with_entity(self.entity), error_handler);
        self
    }

    /// Pushes an [`EntityCommand`] to the queue, which will get executed for the current [`Entity`].
    ///
    /// Unlike [`EntityCommands::queue_handled`], this will completely ignore any errors that occur.
    pub fn queue_silenced<C: EntityCommand<T> + CommandWithEntity<M>, T, M>(
        &mut self,
        command: C,
    ) -> &mut Self {
        self.commands
            .queue_silenced(command.with_entity(self.entity));
        self
    }

    /// Removes all components except the given [`Bundle`] from the entity.
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
    /// fn remove_combat_stats_system(mut commands: Commands, player: Res<PlayerEntity>) {
    ///     commands
    ///         .entity(player.entity)
    ///         // You can retain a pre-defined Bundle of components,
    ///         // with this removing only the Defense component.
    ///         .retain::<CombatBundle>()
    ///         // You can also retain only a single component.
    ///         .retain::<Health>();
    /// }
    /// # bevy_ecs::system::assert_is_system(remove_combat_stats_system);
    /// ```
    #[track_caller]
    pub fn retain<B: Bundle>(&mut self) -> &mut Self {
        self.queue(entity_command::retain::<B>())
    }

    /// Logs the components of the entity at the [`info`](log::info) level.
    pub fn log_components(&mut self) -> &mut Self {
        self.queue(entity_command::log_components())
    }

    /// Returns the underlying [`Commands`].
    pub fn commands(&mut self) -> Commands<'_, '_> {
        self.commands.reborrow()
    }

    /// Returns a mutable reference to the underlying [`Commands`].
    pub fn commands_mut(&mut self) -> &mut Commands<'a, 'a> {
        &mut self.commands
    }

    /// Creates an [`Observer`](crate::observer::Observer) watching for an [`EntityEvent`] of type `E` whose [`EntityEvent::event_target`]
    /// targets this entity.
    pub fn observe<M>(&mut self, observer: impl IntoEntityObserver<M>) -> &mut Self {
        self.queue(entity_command::observe(observer))
    }

    /// Clones parts of an entity (components, observers, etc.) onto another entity,
    /// configured through [`EntityClonerBuilder`].
    ///
    /// The other entity will receive all the components of the original that implement
    /// [`Clone`] or [`Reflect`](bevy_reflect::Reflect) except those that are
    /// [denied](EntityClonerBuilder::deny) in the `config`.
    ///
    /// # Panics
    ///
    /// The command will panic when applied if the target entity does not exist.
    ///
    /// # Example
    ///
    /// Configure through [`EntityClonerBuilder<OptOut>`] as follows:
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component, Clone)]
    /// struct ComponentA(u32);
    /// #[derive(Component, Clone)]
    /// struct ComponentB(u32);
    ///
    /// fn example_system(mut commands: Commands) {
    ///     // Create an empty entity.
    ///     let target = commands.spawn_empty().id();
    ///
    ///     // Create a new entity and keep its EntityCommands.
    ///     let mut entity = commands.spawn((ComponentA(10), ComponentB(20)));
    ///
    ///     // Clone ComponentA but not ComponentB onto the target.
    ///     entity.clone_with_opt_out(target, |builder| {
    ///         builder.deny::<ComponentB>();
    ///     });
    /// }
    /// # bevy_ecs::system::assert_is_system(example_system);
    /// ```
    ///
    /// See [`EntityClonerBuilder`] for more options.
    pub fn clone_with_opt_out(
        &mut self,
        target: Entity,
        config: impl FnOnce(&mut EntityClonerBuilder<OptOut>) + Send + Sync + 'static,
    ) -> &mut Self {
        self.queue(entity_command::clone_with_opt_out(target, config))
    }

    /// Clones parts of an entity (components, observers, etc.) onto another entity,
    /// configured through [`EntityClonerBuilder`].
    ///
    /// The other entity will receive only the components of the original that implement
    /// [`Clone`] or [`Reflect`](bevy_reflect::Reflect) and are
    /// [allowed](EntityClonerBuilder::allow) in the `config`.
    ///
    /// # Panics
    ///
    /// The command will panic when applied if the target entity does not exist.
    ///
    /// # Example
    ///
    /// Configure through [`EntityClonerBuilder<OptIn>`] as follows:
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component, Clone)]
    /// struct ComponentA(u32);
    /// #[derive(Component, Clone)]
    /// struct ComponentB(u32);
    ///
    /// fn example_system(mut commands: Commands) {
    ///     // Create an empty entity.
    ///     let target = commands.spawn_empty().id();
    ///
    ///     // Create a new entity and keep its EntityCommands.
    ///     let mut entity = commands.spawn((ComponentA(10), ComponentB(20)));
    ///
    ///     // Clone ComponentA but not ComponentB onto the target.
    ///     entity.clone_with_opt_in(target, |builder| {
    ///         builder.allow::<ComponentA>();
    ///     });
    /// }
    /// # bevy_ecs::system::assert_is_system(example_system);
    /// ```
    ///
    /// See [`EntityClonerBuilder`] for more options.
    pub fn clone_with_opt_in(
        &mut self,
        target: Entity,
        config: impl FnOnce(&mut EntityClonerBuilder<OptIn>) + Send + Sync + 'static,
    ) -> &mut Self {
        self.queue(entity_command::clone_with_opt_in(target, config))
    }

    /// Spawns a clone of this entity and returns the [`EntityCommands`] of the clone.
    ///
    /// The clone will receive all the components of the original that implement
    /// [`Clone`] or [`Reflect`](bevy_reflect::Reflect).
    ///
    /// To configure cloning behavior (such as only cloning certain components),
    /// use [`EntityCommands::clone_and_spawn_with_opt_out`]/
    /// [`opt_out`](EntityCommands::clone_and_spawn_with_opt_out).
    ///
    /// # Note
    ///
    /// If the original entity does not exist when this command is applied,
    /// the returned entity will have no components.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component, Clone)]
    /// struct ComponentA(u32);
    /// #[derive(Component, Clone)]
    /// struct ComponentB(u32);
    ///
    /// fn example_system(mut commands: Commands) {
    ///     // Create a new entity and store its EntityCommands.
    ///     let mut entity = commands.spawn((ComponentA(10), ComponentB(20)));
    ///
    ///     // Create a clone of the entity.
    ///     let mut entity_clone = entity.clone_and_spawn();
    /// }
    /// # bevy_ecs::system::assert_is_system(example_system);
    pub fn clone_and_spawn(&mut self) -> EntityCommands<'_> {
        self.clone_and_spawn_with_opt_out(|_| {})
    }

    /// Spawns a clone of this entity and allows configuring cloning behavior
    /// using [`EntityClonerBuilder`], returning the [`EntityCommands`] of the clone.
    ///
    /// The clone will receive all the components of the original that implement
    /// [`Clone`] or [`Reflect`](bevy_reflect::Reflect) except those that are
    /// [denied](EntityClonerBuilder::deny) in the `config`.
    ///
    /// See the methods on [`EntityClonerBuilder<OptOut>`] for more options.
    ///
    /// # Note
    ///
    /// If the original entity does not exist when this command is applied,
    /// the returned entity will have no components.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component, Clone)]
    /// struct ComponentA(u32);
    /// #[derive(Component, Clone)]
    /// struct ComponentB(u32);
    ///
    /// fn example_system(mut commands: Commands) {
    ///     // Create a new entity and store its EntityCommands.
    ///     let mut entity = commands.spawn((ComponentA(10), ComponentB(20)));
    ///
    ///     // Create a clone of the entity with ComponentA but without ComponentB.
    ///     let mut entity_clone = entity.clone_and_spawn_with_opt_out(|builder| {
    ///         builder.deny::<ComponentB>();
    ///     });
    /// }
    /// # bevy_ecs::system::assert_is_system(example_system);
    pub fn clone_and_spawn_with_opt_out(
        &mut self,
        config: impl FnOnce(&mut EntityClonerBuilder<OptOut>) + Send + Sync + 'static,
    ) -> EntityCommands<'_> {
        let entity_clone = self.commands().spawn_empty().id();
        self.clone_with_opt_out(entity_clone, config);
        EntityCommands {
            commands: self.commands_mut().reborrow(),
            entity: entity_clone,
        }
    }

    /// Spawns a clone of this entity and allows configuring cloning behavior
    /// using [`EntityClonerBuilder`], returning the [`EntityCommands`] of the clone.
    ///
    /// The clone will receive only the components of the original that implement
    /// [`Clone`] or [`Reflect`](bevy_reflect::Reflect) and are
    /// [allowed](EntityClonerBuilder::allow) in the `config`.
    ///
    /// See the methods on [`EntityClonerBuilder<OptIn>`] for more options.
    ///
    /// # Note
    ///
    /// If the original entity does not exist when this command is applied,
    /// the returned entity will have no components.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component, Clone)]
    /// struct ComponentA(u32);
    /// #[derive(Component, Clone)]
    /// struct ComponentB(u32);
    ///
    /// fn example_system(mut commands: Commands) {
    ///     // Create a new entity and store its EntityCommands.
    ///     let mut entity = commands.spawn((ComponentA(10), ComponentB(20)));
    ///
    ///     // Create a clone of the entity with ComponentA but without ComponentB.
    ///     let mut entity_clone = entity.clone_and_spawn_with_opt_in(|builder| {
    ///         builder.allow::<ComponentA>();
    ///     });
    /// }
    /// # bevy_ecs::system::assert_is_system(example_system);
    pub fn clone_and_spawn_with_opt_in(
        &mut self,
        config: impl FnOnce(&mut EntityClonerBuilder<OptIn>) + Send + Sync + 'static,
    ) -> EntityCommands<'_> {
        let entity_clone = self.commands().spawn_empty().id();
        self.clone_with_opt_in(entity_clone, config);
        EntityCommands {
            commands: self.commands_mut().reborrow(),
            entity: entity_clone,
        }
    }

    /// Clones the specified components of this entity and inserts them into another entity.
    ///
    /// Components can only be cloned if they implement
    /// [`Clone`] or [`Reflect`](bevy_reflect::Reflect).
    ///
    /// # Panics
    ///
    /// The command will panic when applied if the target entity does not exist.
    pub fn clone_components<B: Bundle>(&mut self, target: Entity) -> &mut Self {
        self.queue(entity_command::clone_components::<B>(target))
    }

    /// Moves the specified components of this entity into another entity.
    ///
    /// Components with [`Ignore`] clone behavior will not be moved, while components that
    /// have a [`Custom`] clone behavior will be cloned using it and then removed from the source entity.
    /// All other components will be moved without any other special handling.
    ///
    /// Note that this will trigger `on_remove` hooks/observers on this entity and `on_insert`/`on_add` hooks/observers on the target entity.
    ///
    /// # Panics
    ///
    /// The command will panic when applied if the target entity does not exist.
    ///
    /// [`Ignore`]: crate::component::ComponentCloneBehavior::Ignore
    /// [`Custom`]: crate::component::ComponentCloneBehavior::Custom
    pub fn move_components<B: Bundle>(&mut self, target: Entity) -> &mut Self {
        self.queue(entity_command::move_components::<B>(target))
    }

    /// Passes the current entity into the given function, and triggers the [`EntityEvent`] returned by that function.
    ///
    /// # Example
    ///
    /// A surprising number of functions meet the trait bounds for `event_fn`:
    ///
    /// ```rust
    /// # use bevy_ecs::prelude::*;
    ///
    /// #[derive(EntityEvent)]
    /// struct Explode(Entity);
    ///
    /// impl From<Entity> for Explode {
    ///    fn from(entity: Entity) -> Self {
    ///       Explode(entity)
    ///    }
    /// }
    ///
    ///
    /// fn trigger_via_constructor(mut commands: Commands) {
    ///     // The fact that `Explode` is a single-field tuple struct
    ///     // ensures that `Explode(entity)` is a function that generates
    ///     // an EntityEvent, meeting the trait bounds for `event_fn`.
    ///     commands.spawn_empty().trigger(Explode);
    ///
    /// }
    ///
    ///
    /// fn trigger_via_from_trait(mut commands: Commands) {
    ///     // This variant also works for events like `struct Explode { entity: Entity }`
    ///     commands.spawn_empty().trigger(Explode::from);
    /// }
    ///
    /// fn trigger_via_closure(mut commands: Commands) {
    ///     commands.spawn_empty().trigger(|entity| Explode(entity));
    /// }
    /// ```
    #[track_caller]
    pub fn trigger<'t, E: EntityEvent<Trigger<'t>: Default>>(
        &mut self,
        event_fn: impl FnOnce(Entity) -> E,
    ) -> &mut Self {
        let event = (event_fn)(self.entity);
        self.commands.trigger(event);
        self
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
    /// [Insert](EntityCommands::insert) `default` into this entity,
    /// if `T` is not already present.
    #[track_caller]
    pub fn or_insert(&mut self, default: T) -> &mut Self {
        self.entity_commands.insert_if_new(default);
        self
    }

    /// [Insert](EntityCommands::insert) `default` into this entity,
    /// if `T` is not already present.
    ///
    /// # Note
    ///
    /// If the entity does not exist when this command is executed,
    /// the resulting error will be ignored.
    #[track_caller]
    pub fn or_try_insert(&mut self, default: T) -> &mut Self {
        self.entity_commands.try_insert_if_new(default);
        self
    }

    /// [Insert](EntityCommands::insert) the value returned from `default` into this entity,
    /// if `T` is not already present.
    ///
    /// `default` will only be invoked if the component will actually be inserted.
    #[track_caller]
    pub fn or_insert_with<F>(&mut self, default: F) -> &mut Self
    where
        F: FnOnce() -> T + Send + 'static,
    {
        self.entity_commands
            .queue(entity_command::insert_with(default, InsertMode::Keep));
        self
    }

    /// [Insert](EntityCommands::insert) the value returned from `default` into this entity,
    /// if `T` is not already present.
    ///
    /// `default` will only be invoked if the component will actually be inserted.
    ///
    /// # Note
    ///
    /// If the entity does not exist when this command is executed,
    /// the resulting error will be ignored.
    #[track_caller]
    pub fn or_try_insert_with<F>(&mut self, default: F) -> &mut Self
    where
        F: FnOnce() -> T + Send + 'static,
    {
        self.entity_commands
            .queue_silenced(entity_command::insert_with(default, InsertMode::Keep));
        self
    }

    /// [Insert](EntityCommands::insert) `T::default` into this entity,
    /// if `T` is not already present.
    ///
    /// `T::default` will only be invoked if the component will actually be inserted.
    #[track_caller]
    pub fn or_default(&mut self) -> &mut Self
    where
        T: Default,
    {
        self.or_insert_with(T::default)
    }

    /// [Insert](EntityCommands::insert) `T::from_world` into this entity,
    /// if `T` is not already present.
    ///
    /// `T::from_world` will only be invoked if the component will actually be inserted.
    #[track_caller]
    pub fn or_from_world(&mut self) -> &mut Self
    where
        T: FromWorld,
    {
        self.entity_commands
            .queue(entity_command::insert_from_world::<T>(InsertMode::Keep));
        self
    }

    /// Get the [`EntityCommands`] from which the [`EntityEntryCommands`] was initiated.
    ///
    /// This allows you to continue chaining method calls after calling [`EntityCommands::entry`].
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
    ///         // Modify the component if it exists.
    ///         .and_modify(|mut lvl| lvl.0 += 1)
    ///         // Otherwise, insert a default value.
    ///         .or_insert(Level(0))
    ///         // Return the EntityCommands for the entity.
    ///         .entity()
    ///         // Continue chaining method calls.
    ///         .insert(Name::new("Player"));
    /// }
    /// # bevy_ecs::system::assert_is_system(level_up_system);
    /// ```
    pub fn entity(&mut self) -> EntityCommands<'_> {
        self.entity_commands.reborrow()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        component::Component,
        resource::Resource,
        system::{Commands, SystemState},
        world::{CommandQueue, FromWorld, World},
    };
    use alloc::{string::String, sync::Arc, vec, vec::Vec};
    use bevy_utils::default;
    use core::{
        any::TypeId,
        sync::atomic::{AtomicU8, AtomicUsize, Ordering},
    };

    #[expect(
        dead_code,
        reason = "This struct is used to test how `Drop` behavior works in regards to SparseSet storage, and as such is solely a wrapper around `DropCk` to make it use the SparseSet storage. Because of this, the inner field is intentionally never read."
    )]
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

    #[derive(Resource)]
    struct V<T>(T);

    fn simple_command(world: &mut World) {
        world.spawn((W(0u32), W(42u64)));
    }

    impl FromWorld for W<String> {
        fn from_world(world: &mut World) -> Self {
            let v = world.resource::<V<usize>>();
            Self("*".repeat(v.0))
        }
    }

    impl Default for W<u8> {
        fn default() -> Self {
            unreachable!()
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
        world.insert_resource(V(5_usize));
        let mut commands = Commands::new(&mut queue, &world);
        commands.entity(entity).entry::<W<String>>().or_from_world();
        queue.apply(&mut world);
        assert_eq!("*****", &world.get::<W<String>>(entity).unwrap().0);
        let mut commands = Commands::new(&mut queue, &world);
        let id = commands.entity(entity).entry::<W<u64>>().entity().id();
        queue.apply(&mut world);
        assert_eq!(id, entity);
        let mut commands = Commands::new(&mut queue, &world);
        commands
            .entity(entity)
            .entry::<W<u8>>()
            .or_insert_with(|| W(5))
            .or_insert_with(|| unreachable!())
            .or_try_insert_with(|| unreachable!())
            .or_default()
            .or_from_world();
        queue.apply(&mut world);
        assert_eq!(5, world.get::<W<u8>>(entity).unwrap().0);
    }

    #[test]
    fn commands() {
        let mut world = World::default();
        let mut command_queue = CommandQueue::default();
        let entity = Commands::new(&mut command_queue, &world)
            .spawn((W(1u32), W(2u64)))
            .id();
        command_queue.apply(&mut world);
        assert_eq!(world.query::<&W<u32>>().query(&world).count(), 1);
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
            commands.insert_resource(V(123i32));
            commands.insert_resource(V(456.0f64));
        }

        queue.apply(&mut world);
        assert!(world.contains_resource::<V<i32>>());
        assert!(world.contains_resource::<V<f64>>());

        {
            let mut commands = Commands::new(&mut queue, &world);
            // test resource removal
            commands.remove_resource::<V<i32>>();
        }
        queue.apply(&mut world);
        assert!(!world.contains_resource::<V<i32>>());
        assert!(world.contains_resource::<V<f64>>());
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

        let resources = world.iter_resources().count();
        let id = world.register_system_cached(nothing);
        assert_eq!(world.iter_resources().count(), resources + 1);
        assert!(world.get_entity(id.entity).is_ok());

        let mut commands = Commands::new(&mut queue, &world);
        commands.unregister_system_cached(nothing);
        queue.apply(&mut world);
        assert_eq!(world.iter_resources().count(), resources);
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
            commands.insert_resource(V(123i32));
        }
        let mut queue_2 = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue_2, &world);
            commands.insert_resource(V(456.0f64));
        }
        queue_1.append(&mut queue_2);
        queue_1.apply(&mut world);
        assert!(world.contains_resource::<V<i32>>());
        assert!(world.contains_resource::<V<f64>>());
    }

    #[test]
    fn track_spawn_ticks() {
        let mut world = World::default();
        world.increment_change_tick();
        let expected = world.change_tick();
        let id = world.commands().spawn_empty().id();
        world.flush();
        assert_eq!(
            Some(expected),
            world.entities().entity_get_spawn_or_despawn_tick(id)
        );
    }

    #[test]
    fn command_queues_are_shared_and_ordered() {
        let mut world = World::default();
        let mut system_state = SystemState::<(Commands, Commands)>::new(&mut world);

        let counter: Arc<AtomicU8> = default();

        let (mut commands_a, mut commands_b) = system_state.get(&world);

        commands_a.queue({
            let counter = counter.clone();
            move |_: &mut World| {
                counter.fetch_add(1, Ordering::SeqCst);
            }
        });

        commands_b.queue({
            let counter = counter.clone();
            move |_: &mut World| {
                assert_eq!(1, counter.load(Ordering::SeqCst));
                counter.fetch_add(1, Ordering::SeqCst);
            }
        });

        commands_a.queue({
            let counter = counter.clone();
            move |_: &mut World| {
                assert_eq!(2, counter.load(Ordering::SeqCst));
                counter.fetch_add(1, Ordering::SeqCst);
            }
        });

        commands_b.queue({
            let counter = counter.clone();
            move |_: &mut World| {
                assert_eq!(3, counter.load(Ordering::SeqCst));
            }
        });

        system_state.apply(&mut world);

        assert_eq!(3, counter.load(Ordering::SeqCst));
    }
}
