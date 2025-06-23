#[cfg(feature = "bevy_reflect")]
use crate::reflect::ReflectComponent;
use crate::{
    change_detection::Mut,
    entity::Entity,
    error::BevyError,
    system::{
        input::SystemInput, BoxedSystem, IntoSystem, RunSystemError, SystemParamValidationError,
    },
    world::World,
};
use alloc::boxed::Box;
use bevy_ecs_macros::{Component, Resource};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use core::marker::PhantomData;
use thiserror::Error;

/// A small wrapper for [`BoxedSystem`] that also keeps track whether or not the system has been initialized.
#[derive(Component)]
#[require(SystemIdMarker)]
pub(crate) struct RegisteredSystem<I, O> {
    initialized: bool,
    system: BoxedSystem<I, O>,
}

impl<I, O> RegisteredSystem<I, O> {
    pub fn new(system: BoxedSystem<I, O>) -> Self {
        RegisteredSystem {
            initialized: false,
            system,
        }
    }
}

/// Marker [`Component`](bevy_ecs::component::Component) for identifying [`SystemId`] [`Entity`]s.
#[derive(Component, Default)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Component, Default))]
pub struct SystemIdMarker;

/// A system that has been removed from the registry.
/// It contains the system and whether or not it has been initialized.
///
/// This struct is returned by [`World::unregister_system`].
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
pub struct CachedSystemId<S> {
    /// The cached `SystemId` as an `Entity`.
    pub entity: Entity,
    _marker: PhantomData<fn() -> S>,
}

impl<S> CachedSystemId<S> {
    /// Creates a new `CachedSystemId` struct given a `SystemId`.
    pub fn new<I: SystemInput, O>(id: SystemId<I, O>) -> Self {
        Self {
            entity: id.entity(),
            _marker: PhantomData,
        }
    }
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
    /// [`System`](crate::system::System) trait object and put in a [`Box`].
    pub fn register_boxed_system<I, O>(&mut self, system: BoxedSystem<I, O>) -> SystemId<I, O>
    where
        I: SystemInput + 'static,
        O: 'static,
    {
        let entity = self.spawn(RegisteredSystem::new(system)).id();
        SystemId::from_entity(entity)
    }

    /// Removes a registered system and returns the system, if it exists.
    /// After removing a system, the [`SystemId`] becomes invalid and attempting to use it afterwards will result in errors.
    /// Re-adding the removed system will register it on a new [`SystemId`].
    ///
    /// If no system corresponds to the given [`SystemId`], this method returns an error.
    /// Systems are also not allowed to remove themselves, this returns an error too.
    pub fn unregister_system<I, O>(
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
    /// Also runs any queued-up commands.
    ///
    /// In order to run a chained system with an input, use [`World::run_system_with`] instead.
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
        self.run_system_with(id, ())
    }

    /// Run a stored chained system by its [`SystemId`], providing an input value.
    /// Before running a system, it must first be registered.
    /// The method [`World::register_system`] stores a given system and returns a [`SystemId`].
    ///
    /// Also runs any queued-up commands.
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
    /// assert_eq!(world.run_system_with(counter_one, 1).unwrap(), 1);
    /// assert_eq!(world.run_system_with(counter_one, 20).unwrap(), 21);
    /// assert_eq!(world.run_system_with(counter_two, 30).unwrap(), 30);
    /// ```
    ///
    /// See [`World::run_system`] for more examples.
    pub fn run_system_with<I, O>(
        &mut self,
        id: SystemId<I, O>,
        input: I::Inner<'_>,
    ) -> Result<O, RegisteredSystemError<I, O>>
    where
        I: SystemInput + 'static,
        O: 'static,
    {
        // Lookup
        let mut entity = self
            .get_entity_mut(id.entity)
            .map_err(|_| RegisteredSystemError::SystemIdNotRegistered(id))?;

        // Take ownership of system trait object
        let RegisteredSystem {
            mut initialized,
            mut system,
        } = entity
            .take::<RegisteredSystem<I, O>>()
            .ok_or(RegisteredSystemError::Recursive(id))?;

        // Run the system
        if !initialized {
            system.initialize(self);
            initialized = true;
        }

        // Wait to run the commands until the system is available again.
        // This is needed so the systems can recursively run themselves.
        let result = system.run_without_applying_deferred(input, self);
        system.queue_deferred(self.into());

        // Return ownership of system trait object (if entity still exists)
        if let Ok(mut entity) = self.get_entity_mut(id.entity) {
            entity.insert::<RegisteredSystem<I, O>>(RegisteredSystem {
                initialized,
                system,
            });
        }

        // Run any commands enqueued by the system
        self.flush();
        Ok(result?)
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

        if !self.contains_resource::<CachedSystemId<S>>() {
            let id = self.register_system(system);
            self.insert_resource(CachedSystemId::<S>::new(id));
            return id;
        }

        self.resource_scope(|world, mut id: Mut<CachedSystemId<S>>| {
            if let Ok(mut entity) = world.get_entity_mut(id.entity) {
                if !entity.contains::<RegisteredSystem<I, O>>() {
                    entity.insert(RegisteredSystem::new(Box::new(IntoSystem::into_system(
                        system,
                    ))));
                }
            } else {
                id.entity = world.register_system(system).entity();
            }
            SystemId::from_entity(id.entity)
        })
    }

    /// Removes a cached system and its [`CachedSystemId`] resource.
    ///
    /// See [`World::register_system_cached`] for more information.
    pub fn unregister_system_cached<I, O, M, S>(
        &mut self,
        _system: S,
    ) -> Result<RemovedSystem<I, O>, RegisteredSystemError<I, O>>
    where
        I: SystemInput + 'static,
        O: 'static,
        S: IntoSystem<I, O, M> + 'static,
    {
        let id = self
            .remove_resource::<CachedSystemId<S>>()
            .ok_or(RegisteredSystemError::SystemNotCached)?;
        self.unregister_system(SystemId::<I, O>::from_entity(id.entity))
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
        self.run_system_with(id, input)
    }
}

/// An operation with stored systems failed.
#[derive(Error)]
pub enum RegisteredSystemError<I: SystemInput = (), O = ()> {
    /// A system was run by id, but no system with that id was found.
    ///
    /// Did you forget to register it?
    #[error("System {0:?} was not registered")]
    SystemIdNotRegistered(SystemId<I, O>),
    /// A cached system was removed by value, but no system with its type was found.
    ///
    /// Did you forget to register it?
    #[error("Cached system was not found")]
    SystemNotCached,
    /// A system tried to run itself recursively.
    #[error("System {0:?} tried to run itself recursively")]
    Recursive(SystemId<I, O>),
    /// A system tried to remove itself.
    #[error("System {0:?} tried to remove itself")]
    SelfRemove(SystemId<I, O>),
    /// System could not be run due to parameters that failed validation.
    /// This is not considered an error.
    #[error("System did not run due to failed parameter validation: {0}")]
    Skipped(SystemParamValidationError),
    /// System returned an error or failed required parameter validation.
    #[error("System returned error: {0}")]
    Failed(BevyError),
}

impl<I: SystemInput, O> From<RunSystemError> for RegisteredSystemError<I, O> {
    fn from(value: RunSystemError) -> Self {
        match value {
            RunSystemError::Skipped(err) => Self::Skipped(err),
            RunSystemError::Failed(err) => Self::Failed(err),
        }
    }
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
            Self::Skipped(arg0) => f.debug_tuple("Skipped").field(arg0).finish(),
            Self::Failed(arg0) => f.debug_tuple("Failed").field(arg0).finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::cell::Cell;

    use bevy_utils::default;

    use crate::{prelude::*, system::SystemId};

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
            .run_system_with(id, NonCopy(1))
            .expect("system runs successfully");
        assert_eq!(*world.resource::<Counter>(), Counter(2));

        world
            .run_system_with(id, NonCopy(1))
            .expect("system runs successfully");
        assert_eq!(*world.resource::<Counter>(), Counter(3));

        world
            .run_system_with(id, NonCopy(20))
            .expect("system runs successfully");
        assert_eq!(*world.resource::<Counter>(), Counter(23));

        world
            .run_system_with(id, NonCopy(1))
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
    fn fallible_system() {
        fn sys() -> Result<()> {
            Err("error")?;
            Ok(())
        }

        let mut world = World::new();
        let fallible_system_id = world.register_system(sys);
        let output = world.run_system(fallible_system_id);
        assert!(matches!(output, Ok(Err(_))));
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
                commands.run_system_with(callback.0, callback.1);
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

        let result = world.unregister_system_cached(four);
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
    fn cached_fallible_system() {
        fn sys() -> Result<()> {
            Err("error")?;
            Ok(())
        }

        let mut world = World::new();
        let fallible_system_id = world.register_system_cached(sys);
        let output = world.run_system(fallible_system_id);
        assert!(matches!(output, Ok(Err(_))));
        let output = world.run_system_cached(sys);
        assert!(matches!(output, Ok(Err(_))));
        let output = world.run_system_cached_with(sys, ());
        assert!(matches!(output, Ok(Err(_))));
    }

    #[test]
    fn cached_system_commands() {
        fn sys(mut counter: ResMut<Counter>) {
            counter.0 += 1;
        }

        let mut world = World::new();
        world.insert_resource(Counter(0));
        world.commands().run_system_cached(sys);
        world.flush_commands();
        assert_eq!(world.resource::<Counter>().0, 1);
        world.commands().run_system_cached_with(sys, ());
        world.flush_commands();
        assert_eq!(world.resource::<Counter>().0, 2);
    }

    #[test]
    fn cached_fallible_system_commands() {
        fn sys(mut counter: ResMut<Counter>) -> Result {
            counter.0 += 1;
            Ok(())
        }

        let mut world = World::new();
        world.insert_resource(Counter(0));
        world.commands().run_system_cached(sys);
        world.flush_commands();
        assert_eq!(world.resource::<Counter>().0, 1);
        world.commands().run_system_cached_with(sys, ());
        world.flush_commands();
        assert_eq!(world.resource::<Counter>().0, 2);
    }

    #[test]
    #[should_panic(expected = "This system always fails")]
    fn cached_fallible_system_commands_can_fail() {
        use crate::system::command;
        fn sys() -> Result {
            Err("This system always fails".into())
        }

        let mut world = World::new();
        world.commands().queue(command::run_system_cached(sys));
        world.flush_commands();
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
    fn cached_system_into_same_system_type() {
        struct Foo;
        impl IntoSystem<(), (), ()> for Foo {
            type System = ApplyDeferred;
            fn into_system(_: Self) -> Self::System {
                ApplyDeferred
            }
        }

        struct Bar;
        impl IntoSystem<(), (), ()> for Bar {
            type System = ApplyDeferred;
            fn into_system(_: Self) -> Self::System {
                ApplyDeferred
            }
        }

        let mut world = World::new();
        let foo1 = world.register_system_cached(Foo);
        let foo2 = world.register_system_cached(Foo);
        let bar1 = world.register_system_cached(Bar);
        let bar2 = world.register_system_cached(Bar);

        // The `S: IntoSystem` types are different, so they should be cached
        // as separate systems, even though the `<S as IntoSystem>::System`
        // types / values are the same (`ApplyDeferred`).
        assert_ne!(foo1, bar1);

        // But if the `S: IntoSystem` types are the same, they'll be cached
        // as the same system.
        assert_eq!(foo1, foo2);
        assert_eq!(bar1, bar2);
    }

    #[test]
    fn system_with_input_ref() {
        fn with_ref(InRef(input): InRef<u8>, mut counter: ResMut<Counter>) {
            counter.0 += *input;
        }

        let mut world = World::new();
        world.insert_resource(Counter(0));

        let id = world.register_system(with_ref);
        world.run_system_with(id, &2).unwrap();
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
        world.run_system_with(post_system, &mut event).unwrap();
        assert!(!event.cancelled);

        world.resource_mut::<Counter>().0 = 1;
        world.run_system_with(post_system, &mut event).unwrap();
        assert!(event.cancelled);
    }

    #[test]
    fn run_system_invalid_params() {
        use crate::system::RegisteredSystemError;
        use alloc::string::ToString;

        struct T;
        impl Resource for T {}
        fn system(_: Res<T>) {}

        let mut world = World::new();
        let id = world.register_system(system);
        // This fails because `T` has not been added to the world yet.
        let result = world.run_system(id);

        assert!(matches!(result, Err(RegisteredSystemError::Failed { .. })));
        let expected = "System returned error: Parameter `Res<T>` failed validation: Resource does not exist\n";
        assert!(result.unwrap_err().to_string().contains(expected));
    }

    #[test]
    fn run_system_recursive() {
        std::thread_local! {
            static INVOCATIONS_LEFT: Cell<i32> = const { Cell::new(3) };
            static SYSTEM_ID: Cell<Option<SystemId>> = default();
        }

        fn system(mut commands: Commands) {
            let count = INVOCATIONS_LEFT.get() - 1;
            INVOCATIONS_LEFT.set(count);
            if count > 0 {
                commands.run_system(SYSTEM_ID.get().unwrap());
            }
        }

        let mut world = World::new();
        let id = world.register_system(system);
        SYSTEM_ID.set(Some(id));
        world.run_system(id).unwrap();

        assert_eq!(INVOCATIONS_LEFT.get(), 0);
    }

    #[test]
    fn run_system_exclusive_adapters() {
        let mut world = World::new();
        fn system(_: &mut World) {}
        world.run_system_cached(system).unwrap();
        world.run_system_cached(system.pipe(system)).unwrap();
        world.run_system_cached(system.map(|()| {})).unwrap();
    }
}
