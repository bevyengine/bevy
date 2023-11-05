use crate::entity::Entity;
use crate::system::{BoxedSystem, Command, IntoSystem};
use crate::world::World;
use crate::{self as bevy_ecs};
use bevy_ecs_macros::Component;
use thiserror::Error;

/// A small wrapper for [`BoxedSystem`] that also keeps track whether or not the system has been initialized.
#[derive(Component)]
struct RegisteredSystem<I> {
    initialized: bool,
    system: BoxedSystem<I>,
}

/// A system that has been removed from the registry.
/// It contains the system and whether or not it has been initialized.
///
/// This struct is returned by [`World::remove_system`].
pub struct RemovedSystem<I = ()> {
    initialized: bool,
    system: BoxedSystem<I>,
}

impl RemovedSystem {
    /// Is the system initialized?
    /// A system is initialized the first time it's ran.
    pub fn initialized(&self) -> bool {
        self.initialized
    }

    /// The system removed from the storage.
    pub fn system(self) -> BoxedSystem {
        self.system
    }
}

/// An identifier for a registered system.
///
/// These are opaque identifiers, keyed to a specific [`World`],
/// and are created via [`World::register_system`].
#[derive(Eq)]
pub struct SystemId<I = ()>(Entity, std::marker::PhantomData<I>);

// A manual impl is used because the trait bounds should ignore the `I` phantom parameter.
impl<I> Copy for SystemId<I> {}
// A manual impl is used because the trait bounds should ignore the `I` phantom parameter.
impl<I> Clone for SystemId<I> {
    fn clone(&self) -> Self {
        *self
    }
}
// A manual impl is used because the trait bounds should ignore the `I` phantom parameter.
impl<I> PartialEq for SystemId<I> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}
// A manual impl is used because the trait bounds should ignore the `I` phantom parameter.
impl<I> std::hash::Hash for SystemId<I> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}
impl<I> std::fmt::Debug for SystemId<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The PhantomData field is omitted for simplicity.
        f.debug_tuple("SystemId").field(&self.0).finish()
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
    /// due to its better performance and abillity to run non-conflicting systems simultaneously.
    pub fn register_system<I: 'static, M, S: IntoSystem<I, (), M> + 'static>(
        &mut self,
        system: S,
    ) -> SystemId<I> {
        self.register_boxed_system(Box::new(IntoSystem::into_system(system)))
    }

    /// Similar to [`Self::register_system`], but allows passing in a [`BoxedSystem`].
    ///
    ///  This is useful if the [`IntoSystem`] implementor has already been turned into a
    /// [`System`](crate::system::System) trait object and put in a [`Box`].
    pub fn register_boxed_system<I: 'static>(&mut self, system: BoxedSystem<I>) -> SystemId<I> {
        SystemId(
            self.spawn(RegisteredSystem {
                initialized: false,
                system,
            })
            .id(),
            std::marker::PhantomData,
        )
    }

    /// Removes a registered system and returns the system, if it exists.
    /// After removing a system, the [`SystemId`] becomes invalid and attempting to use it afterwards will result in errors.
    /// Re-adding the removed system will register it on a new [`SystemId`].
    ///
    /// If no system corresponds to the given [`SystemId`], this method returns an error.
    /// Systems are also not allowed to remove themselves, this returns an error too.
    pub fn remove_system<I: 'static>(
        &mut self,
        id: SystemId<I>,
    ) -> Result<RemovedSystem<I>, RegisteredSystemError<I>> {
        match self.get_entity_mut(id.0) {
            Some(mut entity) => {
                let registered_system = entity
                    .take::<RegisteredSystem<I>>()
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
    ///  - Exclusive systems cannot be used.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Resource, Default)]
    /// struct Counter(u8);
    ///
    /// fn increment(mut counter: Local<Counter>) {
    ///    counter.0 += 1;
    ///    println!("{}", counter.0);
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
    /// Change detection:
    ///
    /// ```rust
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
    pub fn run_system(&mut self, id: SystemId) -> Result<(), RegisteredSystemError> {
        self.run_system_with_input(id, ())
    }

    /// Run a stored chained system by its [`SystemId`], providing an input value.
    /// Before running a system, it must first be registered.
    /// The method [`World::register_system`] stores a given system and returns a [`SystemId`].
    ///
    /// # Limitations
    ///
    ///  - Stored systems cannot be recursive, they cannot call themselves through [`Commands::run_system`](crate::system::Commands).
    ///  - Exclusive systems cannot be used.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Resource, Default)]
    /// struct Counter(u8);
    ///
    /// fn increment(In(increment_by): In<u8>, mut counter: Local<Counter>) {
    ///    counter.0 += increment_by;
    ///    println!("{}", counter.0);
    /// }
    ///
    /// let mut world = World::default();
    /// let counter_one = world.register_system(increment);
    /// let counter_two = world.register_system(increment);
    /// world.run_system_with_input(counter_one, 1); // -> 1
    /// world.run_system_with_input(counter_one, 20); // -> 21
    /// world.run_system_with_input(counter_two, 30); // -> 51
    /// ```
    ///
    /// See [`World::run_system`] for more examples.
    pub fn run_system_with_input<I: 'static>(
        &mut self,
        id: SystemId<I>,
        input: I,
    ) -> Result<(), RegisteredSystemError<I>> {
        // lookup
        let mut entity = self
            .get_entity_mut(id.0)
            .ok_or(RegisteredSystemError::SystemIdNotRegistered(id))?;

        // take ownership of system trait object
        let RegisteredSystem {
            mut initialized,
            mut system,
        } = entity
            .take::<RegisteredSystem<I>>()
            .ok_or(RegisteredSystemError::Recursive(id))?;

        // run the system
        if !initialized {
            system.initialize(self);
            initialized = true;
        }
        system.run(input, self);
        system.apply_deferred(self);

        // return ownership of system trait object (if entity still exists)
        if let Some(mut entity) = self.get_entity_mut(id.0) {
            entity.insert::<RegisteredSystem<I>>(RegisteredSystem {
                initialized,
                system,
            });
        }
        Ok(())
    }
}

/// The [`Command`] type for [`World::run_system`].
///
/// This command runs systems in an exclusive and single threaded way.
/// Running slow systems can become a bottleneck.
#[derive(Debug, Clone)]
pub struct RunSystem {
    system_id: SystemId,
}

impl RunSystem {
    /// Creates a new [`Command`] struct, which can be added to [`Commands`](crate::system::Commands)
    pub fn new(system_id: SystemId) -> Self {
        Self { system_id }
    }
}

impl Command for RunSystem {
    #[inline]
    fn apply(self, world: &mut World) {
        let _ = world.run_system(self.system_id);
    }
}

/// An operation with stored systems failed.
#[derive(Error)]
pub enum RegisteredSystemError<I = ()> {
    /// A system was run by id, but no system with that id was found.
    ///
    /// Did you forget to register it?
    #[error("System {0:?} was not registered")]
    SystemIdNotRegistered(SystemId<I>),
    /// A system tried to run itself recursively.
    #[error("System {0:?} tried to run itself recursively")]
    Recursive(SystemId<I>),
    /// A system tried to remove itself.
    #[error("System {0:?} tried to remove itself")]
    SelfRemove(SystemId<I>),
}

impl<I> std::fmt::Debug for RegisteredSystemError<I> {
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
}
