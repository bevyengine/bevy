mod parallel_scope;

use super::{Deferred, IntoObserverSystem, IntoSystem, RegisterSystem, Resource};
use crate::{
    self as bevy_ecs,
    bundle::Bundle,
    component::ComponentId,
    entity::{Entities, Entity},
    event::Event,
    observer::{Observer, TriggerEvent, TriggerTargets},
    system::{RunSystemWithInput, SystemId},
    world::command_queue::RawCommandQueue,
    world::{Command, CommandQueue, EntityWorldMut, FromWorld, World},
};
use bevy_utils::tracing::{error, info};
pub use parallel_scope::*;
use std::marker::PhantomData;

/// A [`Command`] queue to perform structural changes to the [`World`].
///
/// Since each command requires exclusive access to the `World`,
/// all queued commands are automatically applied in sequence
/// when the `apply_deferred` system runs (see [`apply_deferred`] documentation for more details).
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
/// Add `mut commands: Commands` as a function argument to your system to get a copy of this struct that will be applied the next time a copy of [`apply_deferred`] runs.
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
/// behavior using [`Commands::add`], which accepts any type implementing [`Command`].
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
/// [`apply_deferred`]: crate::schedule::apply_deferred
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
            world: &mut bevy_ecs::world::World,
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
            world: &mut bevy_ecs::world::World,
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
        unsafe fn get_param<'w, 's>(
            state: &'s mut Self::State,
            system_meta: &bevy_ecs::system::SystemMeta,
            world: bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell<'w>,
            change_tick: bevy_ecs::component::Tick,
        ) -> Self::Item<'w, 's> {
            let(f0,f1,) =  <(Deferred<'s,CommandQueue> , &'w Entities,)as bevy_ecs::system::SystemParam> ::get_param(&mut state.state,system_meta,world,change_tick);
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
    pub fn get_or_spawn(&mut self, entity: Entity) -> EntityCommands {
        self.add(move |world: &mut World| {
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
    pub fn spawn<T: Bundle>(&mut self, bundle: T) -> EntityCommands {
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
    pub fn spawn_batch<I>(&mut self, bundles_iter: I)
    where
        I: IntoIterator + Send + Sync + 'static,
        I::Item: Bundle,
    {
        self.push(spawn_batch(bundles_iter));
    }

    /// Push a [`Command`] onto the queue.
    pub fn push<C: Command>(&mut self, command: C) {
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
    pub fn insert_or_spawn_batch<I, B>(&mut self, bundles_iter: I)
    where
        I: IntoIterator<Item = (Entity, B)> + Send + Sync + 'static,
        B: Bundle,
    {
        self.push(insert_or_spawn_batch(bundles_iter));
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
        self.push(init_resource::<R>);
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
        self.push(insert_resource(resource));
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
        self.push(remove_resource::<R>);
    }

    /// Runs the system corresponding to the given [`SystemId`].
    /// Systems are ran in an exclusive and single threaded way.
    /// Running slow systems can become a bottleneck.
    ///
    /// Calls [`World::run_system`](World::run_system).
    ///
    /// There is no way to get the output of a system when run as a command, because the
    /// execution of the system happens later. To get the output of a system, use
    /// [`World::run_system`] or [`World::run_system_with_input`] instead of running the system as a command.
    pub fn run_system(&mut self, id: SystemId) {
        self.run_system_with_input(id, ());
    }

    /// Runs the system corresponding to the given [`SystemId`].
    /// Systems are ran in an exclusive and single threaded way.
    /// Running slow systems can become a bottleneck.
    ///
    /// Calls [`World::run_system_with_input`](World::run_system_with_input).
    ///
    /// There is no way to get the output of a system when run as a command, because the
    /// execution of the system happens later. To get the output of a system, use
    /// [`World::run_system`] or [`World::run_system_with_input`] instead of running the system as a command.
    pub fn run_system_with_input<I: 'static + Send>(&mut self, id: SystemId<I>, input: I) {
        self.push(RunSystemWithInput::new_with_input(id, input));
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
    ///         *local_system = Some(commands.register_one_shot_system(increment_counter));
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
    /// #   commands.register_one_shot_system(increment_counter)
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
    pub fn register_one_shot_system<
        I: 'static + Send,
        O: 'static + Send,
        M,
        S: IntoSystem<I, O, M> + 'static,
    >(
        &mut self,
        system: S,
    ) -> SystemId<I, O> {
        let entity = self.spawn_empty().id();
        self.push(RegisterSystem::new(system, entity));
        SystemId::from_entity(entity)
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
        self.push(command);
    }

    /// Sends a "global" [`Trigger`] without any targets. This will run any [`Observer`] of the `event` that
    /// isn't scoped to specific targets.
    pub fn trigger(&mut self, event: impl Event) {
        self.add(TriggerEvent { event, targets: () });
    }

    /// Sends a [`Trigger`] for the given targets. This will run any [`Observer`] of the `event` that
    /// watches those targets.
    pub fn trigger_targets(&mut self, event: impl Event, targets: impl TriggerTargets) {
        self.add(TriggerEvent { event, targets });
    }

    /// Spawn an [`Observer`] and returns the [`EntityCommands`] associated with the entity that stores the observer.  
    pub fn observe<E: Event, B: Bundle, M>(
        &mut self,
        observer: impl IntoObserverSystem<E, B, M>,
    ) -> EntityCommands {
        self.spawn(Observer::new(observer))
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
///     commands.spawn_empty().add(count_name);
///     commands.spawn_empty().add(count_name);
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
    fn apply(self, id: Entity, world: &mut World);
    /// Returns a [`Command`] which executes this [`EntityCommand`] for the given [`Entity`].
    #[must_use = "commands do nothing unless applied to a `World`"]
    fn with_entity(self, id: Entity) -> WithEntity<Marker, Self>
    where
        Self: Sized,
    {
        WithEntity {
            cmd: self,
            id,
            marker: PhantomData,
        }
    }
}

/// Turns an [`EntityCommand`] type into a [`Command`] type.
pub struct WithEntity<Marker, C: EntityCommand<Marker>> {
    cmd: C,
    id: Entity,
    marker: PhantomData<fn() -> Marker>,
}

impl<M, C: EntityCommand<M>> Command for WithEntity<M, C>
where
    M: 'static,
{
    #[inline]
    fn apply(self, world: &mut World) {
        self.cmd.apply(self.id, world);
    }
}

/// A list of commands that will be run to modify an [entity](crate::entity).
pub struct EntityCommands<'a> {
    pub(crate) entity: Entity,
    pub(crate) commands: Commands<'a, 'a>,
}

impl EntityCommands<'_> {
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

    /// Adds a [`Bundle`] of components to the entity.
    ///
    /// This will overwrite any previous value(s) of the same component type.
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
    pub fn insert(&mut self, bundle: impl Bundle) -> &mut Self {
        self.add(insert(bundle))
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
    pub fn try_insert(&mut self, bundle: impl Bundle) -> &mut Self {
        self.add(try_insert(bundle))
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
        self.add(remove::<T>)
    }

    /// Removes a component from the entity.
    pub fn remove_by_id(&mut self, component_id: ComponentId) -> &mut Self {
        self.add(remove_by_id(component_id))
    }

    /// Removes all components associated with the entity.
    pub fn clear(&mut self) -> &mut Self {
        self.add(clear())
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
    pub fn despawn(&mut self) {
        self.add(despawn);
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
    ///     .add(|entity: EntityWorldMut| {
    ///         println!("Executed an EntityCommand for {:?}", entity.id());
    ///     });
    /// # }
    /// # bevy_ecs::system::assert_is_system(my_system);
    /// ```
    pub fn add<M: 'static>(&mut self, command: impl EntityCommand<M>) -> &mut Self {
        self.commands.add(command.with_entity(self.entity));
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
        self.add(retain::<T>)
    }

    /// Logs the components of the entity at the info level.
    ///
    /// # Panics
    ///
    /// The command will panic when applied if the associated entity does not exist.
    pub fn log_components(&mut self) {
        self.add(log_components);
    }

    /// Returns the underlying [`Commands`].
    pub fn commands(&mut self) -> Commands {
        self.commands.reborrow()
    }

    /// Creates an [`Observer`](crate::observer::Observer) listening for a trigger of type `T` that targets this entity.
    pub fn observe<E: Event, B: Bundle, M>(
        &mut self,
        system: impl IntoObserverSystem<E, B, M>,
    ) -> &mut Self {
        self.add(observe(system));
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
fn spawn_batch<I, B>(bundles_iter: I) -> impl Command
where
    I: IntoIterator<Item = B> + Send + Sync + 'static,
    B: Bundle,
{
    move |world: &mut World| {
        world.spawn_batch(bundles_iter);
    }
}

/// A [`Command`] that consumes an iterator to add a series of [`Bundle`]s to a set of entities.
/// If any entities do not already exist in the world, they will be spawned.
///
/// This is more efficient than inserting the bundles individually.
fn insert_or_spawn_batch<I, B>(bundles_iter: I) -> impl Command
where
    I: IntoIterator<Item = (Entity, B)> + Send + Sync + 'static,
    B: Bundle,
{
    move |world: &mut World| {
        if let Err(invalid_entities) = world.insert_or_spawn_batch(bundles_iter) {
            error!(
                "Failed to 'insert or spawn' bundle of type {} into the following invalid entities: {:?}",
                std::any::type_name::<B>(),
                invalid_entities
            );
        }
    }
}

/// A [`Command`] that despawns a specific entity.
/// This will emit a warning if the entity does not exist.
///
/// # Note
///
/// This won't clean up external references to the entity (such as parent-child relationships
/// if you're using `bevy_hierarchy`), which may leave the world in an invalid state.
fn despawn(entity: Entity, world: &mut World) {
    world.despawn(entity);
}

/// An [`EntityCommand`] that adds the components in a [`Bundle`] to an entity.
fn insert<T: Bundle>(bundle: T) -> impl EntityCommand {
    move |entity: Entity, world: &mut World| {
        if let Some(mut entity) = world.get_entity_mut(entity) {
            entity.insert(bundle);
        } else {
            panic!("error[B0003]: Could not insert a bundle (of type `{}`) for entity {:?} because it doesn't exist in this World. See: https://bevyengine.org/learn/errors/#b0003", std::any::type_name::<T>(), entity);
        }
    }
}

/// An [`EntityCommand`] that attempts to add the components in a [`Bundle`] to an entity.
fn try_insert(bundle: impl Bundle) -> impl EntityCommand {
    move |entity, world: &mut World| {
        if let Some(mut entity) = world.get_entity_mut(entity) {
            entity.insert(bundle);
        }
    }
}

/// An [`EntityCommand`] that removes components from an entity.
/// For a [`Bundle`] type `T`, this will remove any components in the bundle.
/// Any components in the bundle that aren't found on the entity will be ignored.
fn remove<T: Bundle>(entity: Entity, world: &mut World) {
    if let Some(mut entity) = world.get_entity_mut(entity) {
        entity.remove::<T>();
    }
}

/// An [`EntityCommand`] that removes components with a provided [`ComponentId`] from an entity.
/// # Panics
///
/// Panics if the provided [`ComponentId`] does not exist in the [`World`].
fn remove_by_id(component_id: ComponentId) -> impl EntityCommand {
    move |entity: Entity, world: &mut World| {
        if let Some(mut entity) = world.get_entity_mut(entity) {
            entity.remove_by_id(component_id);
        }
    }
}

/// An [`EntityCommand`] that removes all components associated with a provided entity.
fn clear() -> impl EntityCommand {
    move |entity: Entity, world: &mut World| {
        if let Some(mut entity) = world.get_entity_mut(entity) {
            entity.clear();
        }
    }
}

/// An [`EntityCommand`] that removes components from an entity.
/// For a [`Bundle`] type `T`, this will remove all components except those in the bundle.
/// Any components in the bundle that aren't found on the entity will be ignored.
fn retain<T: Bundle>(entity: Entity, world: &mut World) {
    if let Some(mut entity_mut) = world.get_entity_mut(entity) {
        entity_mut.retain::<T>();
    }
}

/// A [`Command`] that inserts a [`Resource`] into the world using a value
/// created with the [`FromWorld`] trait.
fn init_resource<R: Resource + FromWorld>(world: &mut World) {
    world.init_resource::<R>();
}

/// A [`Command`] that removes the [resource](Resource) `R` from the world.
fn remove_resource<R: Resource>(world: &mut World) {
    world.remove_resource::<R>();
}

/// A [`Command`] that inserts a [`Resource`] into the world.
fn insert_resource<R: Resource>(resource: R) -> impl Command {
    move |world: &mut World| {
        world.insert_resource(resource);
    }
}

/// [`EntityCommand`] to log the components of a given entity. See [`EntityCommands::log_components`].
fn log_components(entity: Entity, world: &mut World) {
    let debug_infos: Vec<_> = world
        .inspect_entity(entity)
        .into_iter()
        .map(|component_info| component_info.name())
        .collect();
    info!("Entity {entity}: {debug_infos:?}");
}

fn observe<E: Event, B: Bundle, M>(
    observer: impl IntoObserverSystem<E, B, M>,
) -> impl EntityCommand {
    move |entity, world: &mut World| {
        if let Some(mut entity) = world.get_entity_mut(entity) {
            entity.observe(observer);
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use crate::{
        self as bevy_ecs,
        component::Component,
        system::{Commands, Resource},
        world::{CommandQueue, World},
    };
    use std::{
        any::TypeId,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
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
