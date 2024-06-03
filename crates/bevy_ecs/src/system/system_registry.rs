use crate::entity::Entity;
use crate::system::{BoxedSystem, IntoSystem};
use crate::world::{Command, World};
use crate::{self as bevy_ecs};
use bevy_ecs_macros::Component;
use thiserror::Error;

/// A small wrapper for [`BoxedSystem`] that also keeps track whether or not the system has been initialized.
#[derive(Component)]
struct RegisteredSystem<I, O> {
    initialized: bool,
    system: BoxedSystem<I, O>,
}

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
pub struct SystemId<I = (), O = ()> {
    pub(crate) entity: Entity,
    pub(crate) marker: std::marker::PhantomData<fn(I) -> O>,
}

impl<I, O> SystemId<I, O> {
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
            marker: std::marker::PhantomData,
        }
    }
}

impl<I, O> Eq for SystemId<I, O> {}

// A manual impl is used because the trait bounds should ignore the `I` and `O` phantom parameters.
impl<I, O> Copy for SystemId<I, O> {}

// A manual impl is used because the trait bounds should ignore the `I` and `O` phantom parameters.
impl<I, O> Clone for SystemId<I, O> {
    fn clone(&self) -> Self {
        *self
    }
}

// A manual impl is used because the trait bounds should ignore the `I` and `O` phantom parameters.
impl<I, O> PartialEq for SystemId<I, O> {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity && self.marker == other.marker
    }
}

// A manual impl is used because the trait bounds should ignore the `I` and `O` phantom parameters.
impl<I, O> std::hash::Hash for SystemId<I, O> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.entity.hash(state);
    }
}

impl<I, O> std::fmt::Debug for SystemId<I, O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SystemId")
            .field(&self.entity)
            .field(&self.entity)
            .finish()
    }
}

impl World {
    /// Registers a system and returns a [`SystemId`] so it can later be called by [`World::run_system`].
    ///
    /// It's possible to register the same systems more than once, they'll be stored separately.
    ///
    /// This is different from adding systems to a [`Schedule`](crate::schedule::Schedule),
    /// because the [`SystemId`] that is returned can be used anywhere in the [`World`] to run the associated system.
    /// This allows for running systems in a pushed-based fashion.
    /// Using a [`Schedule`](crate::schedule::Schedule) is still preferred for most cases
    /// due to its better performance and ability to run non-conflicting systems simultaneously.
    pub fn register_system<I: 'static, O: 'static, M, S: IntoSystem<I, O, M> + 'static>(
        &mut self,
        system: S,
    ) -> SystemId<I, O> {
        self.register_boxed_system(Box::new(IntoSystem::into_system(system)))
    }

    /// Similar to [`Self::register_system`], but allows passing in a [`BoxedSystem`].
    ///
    ///  This is useful if the [`IntoSystem`] implementor has already been turned into a
    /// [`System`](crate::system::System) trait object and put in a [`Box`].
    pub fn register_boxed_system<I: 'static, O: 'static>(
        &mut self,
        system: BoxedSystem<I, O>,
    ) -> SystemId<I, O> {
        SystemId {
            entity: self
                .spawn(RegisteredSystem {
                    initialized: false,
                    system,
                })
                .id(),
            marker: std::marker::PhantomData,
        }
    }

    /// Removes a registered system and returns the system, if it exists.
    /// After removing a system, the [`SystemId`] becomes invalid and attempting to use it afterwards will result in errors.
    /// Re-adding the removed system will register it on a new [`SystemId`].
    ///
    /// If no system corresponds to the given [`SystemId`], this method returns an error.
    /// Systems are also not allowed to remove themselves, this returns an error too.
    pub fn remove_system<I: 'static, O: 'static>(
        &mut self,
        id: SystemId<I, O>,
    ) -> Result<RemovedSystem<I, O>, RegisteredSystemError<I, O>> {
        match self.get_entity_mut(id.entity) {
            Some(mut entity) => {
                let registered_system = entity
                    .take::<RegisteredSystem<I, O>>()
                    .ok_or(RegisteredSystemError::SelfRemove(id))?;
                entity.despawn();
                Ok(RemovedSystem {
                    initialized: registered_system.initialized,
                    system: registered_system.system,
                })
            }
            None => Err(RegisteredSystemError::SystemIdNotRegistered(id)),
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
    pub fn run_system_with_input<I: 'static, O: 'static>(
        &mut self,
        id: SystemId<I, O>,
        input: I,
    ) -> Result<O, RegisteredSystemError<I, O>> {
        // lookup
        let mut entity = self
            .get_entity_mut(id.entity)
            .ok_or(RegisteredSystemError::SystemIdNotRegistered(id))?;

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
        let result = system.run(input, self);

        // return ownership of system trait object (if entity still exists)
        if let Some(mut entity) = self.get_entity_mut(id.entity) {
            entity.insert::<RegisteredSystem<I, O>>(RegisteredSystem {
                initialized,
                system,
            });
        }
        Ok(result)
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
pub struct RunSystemWithInput<I: 'static> {
    system_id: SystemId<I>,
    input: I,
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
    /// Creates a new [`Command`] struct, which can be added to [`Commands`](crate::system::Commands)
    pub fn new(system_id: SystemId) -> Self {
        Self::new_with_input(system_id, ())
    }
}

impl<I: 'static> RunSystemWithInput<I> {
    /// Creates a new [`Command`] struct, which can be added to [`Commands`](crate::system::Commands)
    /// in order to run the specified system with the provided [`In<_>`](crate::system::In) input value.
    pub fn new_with_input(system_id: SystemId<I>, input: I) -> Self {
        Self { system_id, input }
    }
}

impl<I: 'static + Send> Command for RunSystemWithInput<I> {
    #[inline]
    fn apply(self, world: &mut World) {
        let _ = world.run_system_with_input(self.system_id, self.input);
    }
}

/// The [`Command`] type for registering one shot systems from [Commands](crate::system::Commands).
///
/// This command needs an already boxed system to register, and an already spawned entity
pub struct RegisterSystem<I: 'static, O: 'static> {
    system: BoxedSystem<I, O>,
    entity: Entity,
}

impl<I: 'static, O: 'static> RegisterSystem<I, O> {
    /// Creates a new [Command] struct, which can be added to [Commands](crate::system::Commands)
    pub fn new<M, S: IntoSystem<I, O, M> + 'static>(system: S, entity: Entity) -> Self {
        Self {
            system: Box::new(IntoSystem::into_system(system)),
            entity,
        }
    }
}

impl<I: 'static + Send, O: 'static + Send> Command for RegisterSystem<I, O> {
    fn apply(self, world: &mut World) {
        let _ = world.get_entity_mut(self.entity).map(|mut entity| {
            entity.insert(RegisteredSystem {
                initialized: false,
                system: self.system,
            });
        });
    }
}

/// An operation with stored systems failed.
#[derive(Error)]
pub enum RegisteredSystemError<I = (), O = ()> {
    /// A system was run by id, but no system with that id was found.
    ///
    /// Did you forget to register it?
    #[error("System {0:?} was not registered")]
    SystemIdNotRegistered(SystemId<I, O>),
    /// A system tried to run itself recursively.
    #[error("System {0:?} tried to run itself recursively")]
    Recursive(SystemId<I, O>),
    /// A system tried to remove itself.
    #[error("System {0:?} tried to remove itself")]
    SelfRemove(SystemId<I, O>),
}

impl<I, O> std::fmt::Debug for RegisteredSystemError<I, O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SystemIdNotRegistered(arg0) => {
                f.debug_tuple("SystemIdNotRegistered").field(arg0).finish()
            }
            Self::Recursive(arg0) => f.debug_tuple("Recursive").field(arg0).finish(),
            Self::SelfRemove(arg0) => f.debug_tuple("SelfRemove").field(arg0).finish(),
        }
    }
}

mod tests {
    use crate as bevy_ecs;
    use crate::prelude::*;

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
        struct Callback(SystemId<u8>, u8);

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
}
