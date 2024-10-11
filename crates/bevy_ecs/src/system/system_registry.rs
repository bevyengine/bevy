#[cfg(feature = "bevy_reflect")]
use crate::reflect::ReflectComponent;
use crate::{
    self as bevy_ecs,
    bundle::Bundle,
    change_detection::Mut,
    entity::Entity,
    system::{input::SystemInput, BoxedSystem, IntoSystem, System},
    world::{Command, World},
};
use bevy_ecs_macros::{Component, Resource};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use core::marker::PhantomData;
use derive_more::derive::{Display, Error};

/// A small wrapper for [`BoxedSystem`] that also keeps track whether or not the system has been initialized.
#[derive(Component)]
struct RegisteredSystem<I, O> {
    initialized: bool,
    system: BoxedSystem<I, O>,
}

/// Marker [`Component`](bevy_ecs::component::Component) for identifying [`SystemId`] [`Entity`]s.
#[derive(Component)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Component))]
pub struct SystemIdMarker;

/// A system that has been removed from the registry.
/// It contains the system and whether or not it has been initialized.
///
/// This struct is returned by [`World::remove_system`].
pub struct RemovedSystem<I = (), O = ()> {
    initialized: bool,
    system: BoxedSystem<I, O>,
}

impl<I, O> RemovedSystem<I, O> {
    /// Is the system initialized?
    /// A system is initialized the first time it's ran.
    pub fn initialized(&self) -> bool {
        self.initialized
    }

    /// The system removed from the storage.
    pub fn system(self) -> BoxedSystem<I, O> {
        self.system
    }
}

/// An identifier for a registered system.
///
/// These are opaque identifiers, keyed to a specific [`World`],
/// and are created via [`World::register_system`].
pub struct SystemId<I: SystemInput = (), O = ()> {
    pub(crate) entity: Entity,
    pub(crate) marker: PhantomData<fn(I) -> O>,
}

impl<I: SystemInput, O> SystemId<I, O> {
    /// Transforms a [`SystemId`] into the [`Entity`] that holds the one-shot system's state.
    ///
    /// It's trivial to convert [`SystemId`] into an [`Entity`] since a one-shot system
    /// is really an entity with associated handler function.
    ///
    /// For example, this is useful if you want to assign a name label to a system.
    pub fn entity(self) -> Entity {
        self.entity
    }

    /// Create [`SystemId`] from an [`Entity`]. Useful when you only have entity handles to avoid
    /// adding extra components that have a [`SystemId`] everywhere. To run a system with this ID
    ///  - The entity must be a system
    ///  - The `I` + `O` types must be correct
    pub fn from_entity(entity: Entity) -> Self {
        Self {
            entity,
            marker: PhantomData,
        }
    }
}

impl<I: SystemInput, O> Eq for SystemId<I, O> {}

// A manual impl is used because the trait bounds should ignore the `I` and `O` phantom parameters.
impl<I: SystemInput, O> Copy for SystemId<I, O> {}

// A manual impl is used because the trait bounds should ignore the `I` and `O` phantom parameters.
impl<I: SystemInput, O> Clone for SystemId<I, O> {
    fn clone(&self) -> Self {
        *self
    }
}

// A manual impl is used because the trait bounds should ignore the `I` and `O` phantom parameters.
impl<I: SystemInput, O> PartialEq for SystemId<I, O> {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity && self.marker == other.marker
    }
}

// A manual impl is used because the trait bounds should ignore the `I` and `O` phantom parameters.
impl<I: SystemInput, O> core::hash::Hash for SystemId<I, O> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.entity.hash(state);
    }
}

impl<I: SystemInput, O> core::fmt::Debug for SystemId<I, O> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("SystemId").field(&self.entity).finish()
    }
}

/// A cached [`SystemId`] distinguished by the unique function type of its system.
///
/// This resource is inserted by [`World::register_system_cached`].
#[derive(Resource)]
pub struct CachedSystemId<S: System>(pub SystemId<S::In, S::Out>);

/// Creates a [`Bundle`] for a one-shot system entity.
fn system_bundle<I: 'static, O: 'static>(system: BoxedSystem<I, O>) -> impl Bundle {
    (
        RegisteredSystem {
            initialized: false,
            system,
        },
        SystemIdMarker,
    )
}

impl World {
    /// Registers a system and returns a [`SystemId`] so it can later be called by [`World::run_system`].
    ///
    /// It's possible to register multiple copies of the same system by calling this function
    /// multiple times. If that's not what you want, consider using [`World::register_system_cached`]
    /// instead.
    ///
    /// This is different from adding systems to a [`Schedule`](crate::schedule::Schedule),
    /// because the [`SystemId`] that is returned can be used anywhere in the [`World`] to run the associated system.
    /// This allows for running systems in a pushed-based fashion.
    /// Using a [`Schedule`](crate::schedule::Schedule) is still preferred for most cases
    /// due to its better performance and ability to run non-conflicting systems simultaneously.
    pub fn register_system<I, O, M>(
        &mut self,
        system: impl IntoSystem<I, O, M> + 'static,
    ) -> SystemId<I, O>
    where
        I: SystemInput + 'static,
        O: 'static,
    {
        self.register_boxed_system(Box::new(IntoSystem::into_system(system)))
    }

    /// Similar to [`Self::register_system`], but allows passing in a [`BoxedSystem`].
    ///
    ///  This is useful if the [`IntoSystem`] implementor has already been turned into a
    /// [`System`] trait object and put in a [`Box`].
    pub fn register_boxed_system<I, O>(&mut self, system: BoxedSystem<I, O>) -> SystemId<I, O>
    where
        I: SystemInput + 'static,
        O: 'static,
    {
        let entity = self.spawn(system_bundle(system)).id();
        SystemId::from_entity(entity)
    }

    /// Removes a registered system and returns the system, if it exists.
    /// After removing a system, the [`SystemId`] becomes invalid and attempting to use it afterwards will result in errors.
    /// Re-adding the removed system will register it on a new [`SystemId`].
    ///
    /// If no system corresponds to the given [`SystemId`], this method returns an error.
    /// Systems are also not allowed to remove themselves, this returns an error too.
    pub fn remove_system<I, O>(
        &mut self,
        id: SystemId<I, O>,
    ) -> Result<RemovedSystem<I, O>, RegisteredSystemError<I, O>>
    where
        I: SystemInput + 'static,
        O: 'static,
    {
        match self.get_entity_mut(id.entity) {
            Ok(mut entity) => {
                let registered_system = entity
                    .take::<RegisteredSystem<I, O>>()
                    .ok_or(RegisteredSystemError::SelfRemove(id))?;
                entity.despawn();
                Ok(RemovedSystem {
                    initialized: registered_system.initialized,
                    system: registered_system.system,
                })
            }
            Err(_) => Err(RegisteredSystemError::SystemIdNotRegistered(id)),
        }
    }

    /// Run stored systems by their [`SystemId`].
    /// Before running a system, it must first be registered.
    /// The method [`World::register_system`] stores a given system and returns a [`SystemId`].
    /// This is different from [`RunSystemOnce::run_system_once`](crate::system::RunSystemOnce::run_system_once),
    /// because it keeps local state between calls and change detection works correctly.
    ///
    /// In order to run a chained system with an input, use [`World::run_system_with_input`] instead.
    ///
    /// # Limitations
    ///
    ///  - Stored systems cannot be recursive, they cannot call themselves through [`Commands::run_system`](crate::system::Commands).
    ///
    /// # Examples
    ///
    /// ## Running a system
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// fn increment(mut counter: Local<u8>) {
    ///    *counter += 1;
    ///    println!("{}", *counter);
    /// }
    ///
    /// let mut world = World::default();
    /// let counter_one = world.register_system(increment);
    /// let counter_two = world.register_system(increment);
    /// world.run_system(counter_one); // -> 1
    /// world.run_system(counter_one); // -> 2
    /// world.run_system(counter_two); // -> 1
    /// ```
    ///
    /// ## Change detection
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Resource, Default)]
    /// struct ChangeDetector;
    ///
    /// let mut world = World::default();
    /// world.init_resource::<ChangeDetector>();
    /// let detector = world.register_system(|change_detector: ResMut<ChangeDetector>| {
    ///     if change_detector.is_changed() {
    ///         println!("Something happened!");
    ///     } else {
    ///         println!("Nothing happened.");
    ///     }
    /// });
    ///
    /// // Resources are changed when they are first added
    /// let _ = world.run_system(detector); // -> Something happened!
    /// let _ = world.run_system(detector); // -> Nothing happened.
    /// world.resource_mut::<ChangeDetector>().set_changed();
    /// let _ = world.run_system(detector); // -> Something happened!
    /// ```
    ///
    /// ## Getting system output
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    ///
    /// #[derive(Resource)]
    /// struct PlayerScore(i32);
    ///
    /// #[derive(Resource)]
    /// struct OpponentScore(i32);
    ///
    /// fn get_player_score(player_score: Res<PlayerScore>) -> i32 {
    ///   player_score.0
    /// }
    ///
    /// fn get_opponent_score(opponent_score: Res<OpponentScore>) -> i32 {
    ///   opponent_score.0
    /// }
    ///
    /// let mut world = World::default();
    /// world.insert_resource(PlayerScore(3));
    /// world.insert_resource(OpponentScore(2));
    ///
    /// let scoring_systems = [
    ///   ("player", world.register_system(get_player_score)),
    ///   ("opponent", world.register_system(get_opponent_score)),
    /// ];
    ///
    /// for (label, scoring_system) in scoring_systems {
    ///   println!("{label} has score {}", world.run_system(scoring_system).expect("system succeeded"));
    /// }
    /// ```
    pub fn run_system<O: 'static>(
        &mut self,
        id: SystemId<(), O>,
    ) -> Result<O, RegisteredSystemError<(), O>> {
        self.run_system_with_input(id, ())
    }

    /// Run a stored chained system by its [`SystemId`], providing an input value.
    /// Before running a system, it must first be registered.
    /// The method [`World::register_system`] stores a given system and returns a [`SystemId`].
    ///
    /// # Limitations
    ///
    ///  - Stored systems cannot be recursive, they cannot call themselves through [`Commands::run_system`](crate::system::Commands).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// fn increment(In(increment_by): In<u8>, mut counter: Local<u8>) -> u8 {
    ///   *counter += increment_by;
    ///   *counter
    /// }
    ///
    /// let mut world = World::default();
    /// let counter_one = world.register_system(increment);
    /// let counter_two = world.register_system(increment);
    /// assert_eq!(world.run_system_with_input(counter_one, 1).unwrap(), 1);
    /// assert_eq!(world.run_system_with_input(counter_one, 20).unwrap(), 21);
    /// assert_eq!(world.run_system_with_input(counter_two, 30).unwrap(), 30);
    /// ```
    ///
    /// See [`World::run_system`] for more examples.
    pub fn run_system_with_input<I, O>(
        &mut self,
        id: SystemId<I, O>,
        input: I::Inner<'_>,
    ) -> Result<O, RegisteredSystemError<I, O>>
    where
        I: SystemInput + 'static,
        O: 'static,
    {
        // lookup
        let mut entity = self
            .get_entity_mut(id.entity)
            .map_err(|_| RegisteredSystemError::SystemIdNotRegistered(id))?;

        // take ownership of system trait object
        let RegisteredSystem {
            mut initialized,
            mut system,
        } = entity
            .take::<RegisteredSystem<I, O>>()
            .ok_or(RegisteredSystemError::Recursive(id))?;

        // run the system
        if !initialized {
            system.initialize(self);
            initialized = true;
        }

        let result = if system.validate_param(self) {
            Ok(system.run(input, self))
        } else {
            Err(RegisteredSystemError::InvalidParams(id))
        };

        // return ownership of system trait object (if entity still exists)
        if let Ok(mut entity) = self.get_entity_mut(id.entity) {
            entity.insert::<RegisteredSystem<I, O>>(RegisteredSystem {
                initialized,
                system,
            });
        }
        result
    }

    /// Registers a system or returns its cached [`SystemId`].
    ///
    /// If you want to run the system immediately and you don't need its `SystemId`, see
    /// [`World::run_system_cached`].
    ///
    /// The first time this function is called for a particular system, it will register it and
    /// store its [`SystemId`] in a [`CachedSystemId`] resource for later. If you would rather
    /// manage the `SystemId` yourself, or register multiple copies of the same system, use
    /// [`World::register_system`] instead.
    ///
    /// # Limitations
    ///
    /// This function only accepts ZST (zero-sized) systems to guarantee that any two systems of
    /// the same type must be equal. This means that closures that capture the environment, and
    /// function pointers, are not accepted.
    ///
    /// If you want to access values from the environment within a system, consider passing them in
    /// as inputs via [`World::run_system_cached_with`]. If that's not an option, consider
    /// [`World::register_system`] instead.
    pub fn register_system_cached<I, O, M, S>(&mut self, system: S) -> SystemId<I, O>
    where
        I: SystemInput + 'static,
        O: 'static,
        S: IntoSystem<I, O, M> + 'static,
    {
        const {
            assert!(
                size_of::<S>() == 0,
                "Non-ZST systems (e.g. capturing closures, function pointers) cannot be cached.",
            );
        }

        if !self.contains_resource::<CachedSystemId<S::System>>() {
            let id = self.register_system(system);
            self.insert_resource(CachedSystemId::<S::System>(id));
            return id;
        }

        self.resource_scope(|world, mut id: Mut<CachedSystemId<S::System>>| {
            if let Ok(mut entity) = world.get_entity_mut(id.0.entity()) {
                if !entity.contains::<RegisteredSystem<I, O>>() {
                    entity.insert(system_bundle(Box::new(IntoSystem::into_system(system))));
                }
            } else {
                id.0 = world.register_system(system);
            }
            id.0
        })
    }

    /// Removes a cached system and its [`CachedSystemId`] resource.
    ///
    /// See [`World::register_system_cached`] for more information.
    pub fn remove_system_cached<I, O, M, S>(
        &mut self,
        _system: S,
    ) -> Result<RemovedSystem<I, O>, RegisteredSystemError<I, O>>
    where
        I: SystemInput + 'static,
        O: 'static,
        S: IntoSystem<I, O, M> + 'static,
    {
        let id = self
            .remove_resource::<CachedSystemId<S::System>>()
            .ok_or(RegisteredSystemError::SystemNotCached)?;
        self.remove_system(id.0)
    }

    /// Runs a cached system, registering it if necessary.
    ///
    /// See [`World::register_system_cached`] for more information.
    pub fn run_system_cached<O: 'static, M, S: IntoSystem<(), O, M> + 'static>(
        &mut self,
        system: S,
    ) -> Result<O, RegisteredSystemError<(), O>> {
        self.run_system_cached_with(system, ())
    }

    /// Runs a cached system with an input, registering it if necessary.
    ///
    /// See [`World::register_system_cached`] for more information.
    pub fn run_system_cached_with<I, O, M, S>(
        &mut self,
        system: S,
        input: I::Inner<'_>,
    ) -> Result<O, RegisteredSystemError<I, O>>
    where
        I: SystemInput + 'static,
        O: 'static,
        S: IntoSystem<I, O, M> + 'static,
    {
        let id = self.register_system_cached(system);
        self.run_system_with_input(id, input)
    }
}

/// The [`Command`] type for [`World::run_system`] or [`World::run_system_with_input`].
///
/// This command runs systems in an exclusive and single threaded way.
/// Running slow systems can become a bottleneck.
///
/// If the system needs an [`In<_>`](crate::system::In) input value to run, it must
/// be provided as part of the command.
///
/// There is no way to get the output of a system when run as a command, because the
/// execution of the system happens later. To get the output of a system, use
/// [`World::run_system`] or [`World::run_system_with_input`] instead of running the system as a command.
#[derive(Debug, Clone)]
pub struct RunSystemWithInput<I: SystemInput + 'static> {
    system_id: SystemId<I>,
    input: I::Inner<'static>,
}

/// The [`Command`] type for [`World::run_system`].
///
/// This command runs systems in an exclusive and single threaded way.
/// Running slow systems can become a bottleneck.
///
/// If the system needs an [`In<_>`](crate::system::In) input value to run, use the
/// [`RunSystemWithInput`] type instead.
///
/// There is no way to get the output of a system when run as a command, because the
/// execution of the system happens later. To get the output of a system, use
/// [`World::run_system`] or [`World::run_system_with_input`] instead of running the system as a command.
pub type RunSystem = RunSystemWithInput<()>;

impl RunSystem {
    /// Creates a new [`Command`] struct, which can be added to [`Commands`](crate::system::Commands).
    pub fn new(system_id: SystemId) -> Self {
        Self::new_with_input(system_id, ())
    }
}

impl<I: SystemInput + 'static> RunSystemWithInput<I> {
    /// Creates a new [`Command`] struct, which can be added to [`Commands`](crate::system::Commands)
    /// in order to run the specified system with the provided [`In<_>`](crate::system::In) input value.
    pub fn new_with_input(system_id: SystemId<I>, input: I::Inner<'static>) -> Self {
        Self { system_id, input }
    }
}

impl<I> Command for RunSystemWithInput<I>
where
    I: SystemInput<Inner<'static>: Send> + 'static,
{
    #[inline]
    fn apply(self, world: &mut World) {
        _ = world.run_system_with_input(self.system_id, self.input);
    }
}

/// The [`Command`] type for registering one shot systems from [`Commands`](crate::system::Commands).
///
/// This command needs an already boxed system to register, and an already spawned entity.
pub struct RegisterSystem<I: SystemInput + 'static, O: 'static> {
    system: BoxedSystem<I, O>,
    entity: Entity,
}

impl<I, O> RegisterSystem<I, O>
where
    I: SystemInput + 'static,
    O: 'static,
{
    /// Creates a new [`Command`] struct, which can be added to [`Commands`](crate::system::Commands).
    pub fn new<M, S: IntoSystem<I, O, M> + 'static>(system: S, entity: Entity) -> Self {
        Self {
            system: Box::new(IntoSystem::into_system(system)),
            entity,
        }
    }
}

impl<I, O> Command for RegisterSystem<I, O>
where
    I: SystemInput + Send + 'static,
    O: Send + 'static,
{
    fn apply(self, world: &mut World) {
        if let Ok(mut entity) = world.get_entity_mut(self.entity) {
            entity.insert(system_bundle(self.system));
        }
    }
}

/// The [`Command`] type for running a cached one-shot system from
/// [`Commands`](crate::system::Commands).
///
/// See [`World::register_system_cached`] for more information.
pub struct RunSystemCachedWith<S, I, O, M>
where
    I: SystemInput,
    S: IntoSystem<I, O, M>,
{
    system: S,
    input: I::Inner<'static>,
    _phantom: PhantomData<(fn() -> O, fn() -> M)>,
}

impl<S, I, O, M> RunSystemCachedWith<S, I, O, M>
where
    I: SystemInput,
    S: IntoSystem<I, O, M>,
{
    /// Creates a new [`Command`] struct, which can be added to
    /// [`Commands`](crate::system::Commands).
    pub fn new(system: S, input: I::Inner<'static>) -> Self {
        Self {
            system,
            input,
            _phantom: PhantomData,
        }
    }
}

impl<S, I, O, M> Command for RunSystemCachedWith<S, I, O, M>
where
    I: SystemInput<Inner<'static>: Send> + Send + 'static,
    O: Send + 'static,
    S: IntoSystem<I, O, M> + Send + 'static,
    M: 'static,
{
    fn apply(self, world: &mut World) {
        let _ = world.run_system_cached_with(self.system, self.input);
    }
}

/// An operation with stored systems failed.
#[derive(Error, Display)]
pub enum RegisteredSystemError<I: SystemInput = (), O = ()> {
    /// A system was run by id, but no system with that id was found.
    ///
    /// Did you forget to register it?
    #[display("System {_0:?} was not registered")]
    SystemIdNotRegistered(SystemId<I, O>),
    /// A cached system was removed by value, but no system with its type was found.
    ///
    /// Did you forget to register it?
    #[display("Cached system was not found")]
    SystemNotCached,
    /// A system tried to run itself recursively.
    #[display("System {_0:?} tried to run itself recursively")]
    Recursive(SystemId<I, O>),
    /// A system tried to remove itself.
    #[display("System {_0:?} tried to remove itself")]
    SelfRemove(SystemId<I, O>),
    /// System could not be run due to parameters that failed validation.
    ///
    /// This can occur because the data required by the system was not present in the world.
    #[display("The data required by the system {_0:?} was not found in the world and the system did not run due to failed parameter validation.")]
    InvalidParams(SystemId<I, O>),
}

impl<I: SystemInput, O> core::fmt::Debug for RegisteredSystemError<I, O> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SystemIdNotRegistered(arg0) => {
                f.debug_tuple("SystemIdNotRegistered").field(arg0).finish()
            }
            Self::SystemNotCached => write!(f, "SystemNotCached"),
            Self::Recursive(arg0) => f.debug_tuple("Recursive").field(arg0).finish(),
            Self::SelfRemove(arg0) => f.debug_tuple("SelfRemove").field(arg0).finish(),
            Self::InvalidParams(arg0) => f.debug_tuple("InvalidParams").field(arg0).finish(),
        }
    }
}

mod tests {
    use crate::prelude::*;
    use crate::{self as bevy_ecs};

    #[derive(Resource, Default, PartialEq, Debug)]
    struct Counter(u8);

    #[test]
    fn change_detection() {
        #[derive(Resource, Default)]
        struct ChangeDetector;

        fn count_up_iff_changed(
            mut counter: ResMut<Counter>,
            change_detector: ResMut<ChangeDetector>,
        ) {
            if change_detector.is_changed() {
                counter.0 += 1;
            }
        }

        let mut world = World::new();
        world.init_resource::<ChangeDetector>();
        world.init_resource::<Counter>();
        assert_eq!(*world.resource::<Counter>(), Counter(0));
        // Resources are changed when they are first added.
        let id = world.register_system(count_up_iff_changed);
        world.run_system(id).expect("system runs successfully");
        assert_eq!(*world.resource::<Counter>(), Counter(1));
        // Nothing changed
        world.run_system(id).expect("system runs successfully");
        assert_eq!(*world.resource::<Counter>(), Counter(1));
        // Making a change
        world.resource_mut::<ChangeDetector>().set_changed();
        world.run_system(id).expect("system runs successfully");
        assert_eq!(*world.resource::<Counter>(), Counter(2));
    }

    #[test]
    fn local_variables() {
        // The `Local` begins at the default value of 0
        fn doubling(last_counter: Local<Counter>, mut counter: ResMut<Counter>) {
            counter.0 += last_counter.0 .0;
            last_counter.0 .0 = counter.0;
        }

        let mut world = World::new();
        world.insert_resource(Counter(1));
        assert_eq!(*world.resource::<Counter>(), Counter(1));
        let id = world.register_system(doubling);
        world.run_system(id).expect("system runs successfully");
        assert_eq!(*world.resource::<Counter>(), Counter(1));
        world.run_system(id).expect("system runs successfully");
        assert_eq!(*world.resource::<Counter>(), Counter(2));
        world.run_system(id).expect("system runs successfully");
        assert_eq!(*world.resource::<Counter>(), Counter(4));
        world.run_system(id).expect("system runs successfully");
        assert_eq!(*world.resource::<Counter>(), Counter(8));
    }

    #[test]
    fn input_values() {
        // Verify that a non-Copy, non-Clone type can be passed in.
        struct NonCopy(u8);

        fn increment_sys(In(NonCopy(increment_by)): In<NonCopy>, mut counter: ResMut<Counter>) {
            counter.0 += increment_by;
        }

        let mut world = World::new();

        let id = world.register_system(increment_sys);

        // Insert the resource after registering the system.
        world.insert_resource(Counter(1));
        assert_eq!(*world.resource::<Counter>(), Counter(1));

        world
            .run_system_with_input(id, NonCopy(1))
            .expect("system runs successfully");
        assert_eq!(*world.resource::<Counter>(), Counter(2));

        world
            .run_system_with_input(id, NonCopy(1))
            .expect("system runs successfully");
        assert_eq!(*world.resource::<Counter>(), Counter(3));

        world
            .run_system_with_input(id, NonCopy(20))
            .expect("system runs successfully");
        assert_eq!(*world.resource::<Counter>(), Counter(23));

        world
            .run_system_with_input(id, NonCopy(1))
            .expect("system runs successfully");
        assert_eq!(*world.resource::<Counter>(), Counter(24));
    }

    #[test]
    fn output_values() {
        // Verify that a non-Copy, non-Clone type can be returned.
        #[derive(Eq, PartialEq, Debug)]
        struct NonCopy(u8);

        fn increment_sys(mut counter: ResMut<Counter>) -> NonCopy {
            counter.0 += 1;
            NonCopy(counter.0)
        }

        let mut world = World::new();

        let id = world.register_system(increment_sys);

        // Insert the resource after registering the system.
        world.insert_resource(Counter(1));
        assert_eq!(*world.resource::<Counter>(), Counter(1));

        let output = world.run_system(id).expect("system runs successfully");
        assert_eq!(*world.resource::<Counter>(), Counter(2));
        assert_eq!(output, NonCopy(2));

        let output = world.run_system(id).expect("system runs successfully");
        assert_eq!(*world.resource::<Counter>(), Counter(3));
        assert_eq!(output, NonCopy(3));
    }

    #[test]
    fn exclusive_system() {
        let mut world = World::new();
        let exclusive_system_id = world.register_system(|world: &mut World| {
            world.spawn_empty();
        });
        let entity_count = world.entities.len();
        let _ = world.run_system(exclusive_system_id);
        assert_eq!(world.entities.len(), entity_count + 1);
    }

    #[test]
    fn nested_systems() {
        use crate::system::SystemId;

        #[derive(Component)]
        struct Callback(SystemId);

        fn nested(query: Query<&Callback>, mut commands: Commands) {
            for callback in query.iter() {
                commands.run_system(callback.0);
            }
        }

        let mut world = World::new();
        world.insert_resource(Counter(0));

        let increment_two = world.register_system(|mut counter: ResMut<Counter>| {
            counter.0 += 2;
        });
        let increment_three = world.register_system(|mut counter: ResMut<Counter>| {
            counter.0 += 3;
        });
        let nested_id = world.register_system(nested);

        world.spawn(Callback(increment_two));
        world.spawn(Callback(increment_three));
        let _ = world.run_system(nested_id);
        assert_eq!(*world.resource::<Counter>(), Counter(5));
    }

    #[test]
    fn nested_systems_with_inputs() {
        use crate::system::SystemId;

        #[derive(Component)]
        struct Callback(SystemId<In<u8>>, u8);

        fn nested(query: Query<&Callback>, mut commands: Commands) {
            for callback in query.iter() {
                commands.run_system_with_input(callback.0, callback.1);
            }
        }

        let mut world = World::new();
        world.insert_resource(Counter(0));

        let increment_by =
            world.register_system(|In(amt): In<u8>, mut counter: ResMut<Counter>| {
                counter.0 += amt;
            });
        let nested_id = world.register_system(nested);

        world.spawn(Callback(increment_by, 2));
        world.spawn(Callback(increment_by, 3));
        let _ = world.run_system(nested_id);
        assert_eq!(*world.resource::<Counter>(), Counter(5));
    }

    #[test]
    fn cached_system() {
        use crate::system::RegisteredSystemError;

        fn four() -> i32 {
            4
        }

        let mut world = World::new();
        let old = world.register_system_cached(four);
        let new = world.register_system_cached(four);
        assert_eq!(old, new);

        let result = world.remove_system_cached(four);
        assert!(result.is_ok());
        let new = world.register_system_cached(four);
        assert_ne!(old, new);

        let output = world.run_system(old);
        assert!(matches!(
            output,
            Err(RegisteredSystemError::SystemIdNotRegistered(x)) if x == old,
        ));
        let output = world.run_system(new);
        assert!(matches!(output, Ok(x) if x == four()));
        let output = world.run_system_cached(four);
        assert!(matches!(output, Ok(x) if x == four()));
        let output = world.run_system_cached_with(four, ());
        assert!(matches!(output, Ok(x) if x == four()));
    }

    #[test]
    fn cached_system_commands() {
        fn sys(mut counter: ResMut<Counter>) {
            counter.0 = 1;
        }

        let mut world = World::new();
        world.insert_resource(Counter(0));

        world.commands().run_system_cached(sys);
        world.flush_commands();

        assert_eq!(world.resource::<Counter>().0, 1);
    }

    #[test]
    fn cached_system_adapters() {
        fn four() -> i32 {
            4
        }

        fn double(In(i): In<i32>) -> i32 {
            i * 2
        }

        let mut world = World::new();

        let output = world.run_system_cached(four.pipe(double));
        assert!(matches!(output, Ok(8)));

        let output = world.run_system_cached(four.map(|i| i * 2));
        assert!(matches!(output, Ok(8)));
    }

    #[test]
    fn system_with_input_ref() {
        fn with_ref(InRef(input): InRef<u8>, mut counter: ResMut<Counter>) {
            counter.0 += *input;
        }

        let mut world = World::new();
        world.insert_resource(Counter(0));

        let id = world.register_system(with_ref);
        world.run_system_with_input(id, &2).unwrap();
        assert_eq!(*world.resource::<Counter>(), Counter(2));
    }

    #[test]
    fn system_with_input_mut() {
        #[derive(Event)]
        struct MyEvent {
            cancelled: bool,
        }

        fn post(InMut(event): InMut<MyEvent>, counter: ResMut<Counter>) {
            if counter.0 > 0 {
                event.cancelled = true;
            }
        }

        let mut world = World::new();
        world.insert_resource(Counter(0));
        let post_system = world.register_system(post);

        let mut event = MyEvent { cancelled: false };
        world
            .run_system_with_input(post_system, &mut event)
            .unwrap();
        assert!(!event.cancelled);

        world.resource_mut::<Counter>().0 = 1;
        world
            .run_system_with_input(post_system, &mut event)
            .unwrap();
        assert!(event.cancelled);
    }

    #[test]
    fn run_system_invalid_params() {
        use crate::system::RegisteredSystemError;

        struct T;
        impl Resource for T {}
        fn system(_: Res<T>) {}

        let mut world = World::new();
        let id = world.register_system_cached(system);
        // This fails because `T` has not been added to the world yet.
        let result = world.run_system(id);

        assert!(matches!(
            result,
            Err(RegisteredSystemError::InvalidParams(_))
        ));
    }
}
