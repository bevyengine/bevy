use crate::{
    archetype::ArchetypeComponentId,
    component::ComponentId,
    query::Access,
    system::{IntoSystem, System},
    world::World,
};
use std::borrow::Cow;

/// A [`System`] that chains two systems together, creating a new system that routes the output of
/// the first system into the input of the second system, yielding the output of the second system.
///
/// Given two systems `A` and `B`, A may be chained with `B` as `A.chain(B)` if the output type of `A` is
/// equal to the input type of `B`.
///
/// Note that for [`FunctionSystem`](crate::system::FunctionSystem)s the output is the return value
/// of the function and the input is the first [`SystemParam`](crate::system::SystemParam) if it is
/// tagged with [`In`](crate::system::In) or `()` if the function has no designated input parameter.
///
/// # Examples
///
/// ```
/// use std::num::ParseIntError;
///
/// use bevy_ecs::prelude::*;
///
/// fn main() {
///     let mut world = World::default();
///     world.insert_resource(Message("42".to_string()));
///
///     // chain the `parse_message_system`'s output into the `filter_system`s input
///     let mut chained_system = parse_message_system.chain(filter_system);
///     chained_system.initialize(&mut world);
///     assert_eq!(chained_system.run((), &mut world), Some(42));
/// }
///
/// #[derive(Resource)]
/// struct Message(String);
///
/// fn parse_message_system(message: Res<Message>) -> Result<usize, ParseIntError> {
///     message.0.parse::<usize>()
/// }
///
/// fn filter_system(In(result): In<Result<usize, ParseIntError>>) -> Option<usize> {
///     result.ok().filter(|&n| n < 100)
/// }
/// ```
pub struct ChainSystem<SystemA, SystemB> {
    system_a: SystemA,
    system_b: SystemB,
    name: Cow<'static, str>,
    component_access: Access<ComponentId>,
    archetype_component_access: Access<ArchetypeComponentId>,
}

impl<SystemA: System, SystemB: System<In = SystemA::Out>> System for ChainSystem<SystemA, SystemB> {
    type In = SystemA::In;
    type Out = SystemB::Out;

    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        &self.archetype_component_access
    }

    fn component_access(&self) -> &Access<ComponentId> {
        &self.component_access
    }

    fn is_send(&self) -> bool {
        self.system_a.is_send() && self.system_b.is_send()
    }

    unsafe fn run_unsafe(&mut self, input: Self::In, world: &World) -> Self::Out {
        let out = self.system_a.run_unsafe(input, world);
        self.system_b.run_unsafe(out, world)
    }

    fn apply_buffers(&mut self, world: &mut World) {
        self.system_a.apply_buffers(world);
        self.system_b.apply_buffers(world);
    }

    fn initialize(&mut self, world: &mut World) {
        self.system_a.initialize(world);
        self.system_b.initialize(world);
        self.component_access
            .extend(self.system_a.component_access());
        self.component_access
            .extend(self.system_b.component_access());
    }

    fn update_archetype_component_access(&mut self, world: &World) {
        self.system_a.update_archetype_component_access(world);
        self.system_b.update_archetype_component_access(world);

        self.archetype_component_access
            .extend(self.system_a.archetype_component_access());
        self.archetype_component_access
            .extend(self.system_b.archetype_component_access());
    }

    fn check_change_tick(&mut self, change_tick: u32) {
        self.system_a.check_change_tick(change_tick);
        self.system_b.check_change_tick(change_tick);
    }

    fn get_last_change_tick(&self) -> u32 {
        self.system_a.get_last_change_tick()
    }

    fn set_last_change_tick(&mut self, last_change_tick: u32) {
        self.system_a.set_last_change_tick(last_change_tick);
        self.system_b.set_last_change_tick(last_change_tick);
    }
}

/// An extension trait providing the [`IntoChainSystem::chain`] method for convenient [`System`]
/// chaining.
///
/// This trait is blanket implemented for all system pairs that fulfill the chaining requirement.
///
/// See [`ChainSystem`].
pub trait IntoChainSystem<ParamA, Payload, SystemB, ParamB, Out>:
    IntoSystem<(), Payload, ParamA> + Sized
where
    SystemB: IntoSystem<Payload, Out, ParamB>,
{
    /// Chain this system `A` with another system `B` creating a new system that feeds system A's
    /// output into system `B`, returning the output of system `B`.
    fn chain(self, system: SystemB) -> ChainSystem<Self::System, SystemB::System>;
}

impl<SystemA, ParamA, Payload, SystemB, ParamB, Out>
    IntoChainSystem<ParamA, Payload, SystemB, ParamB, Out> for SystemA
where
    SystemA: IntoSystem<(), Payload, ParamA>,
    SystemB: IntoSystem<Payload, Out, ParamB>,
{
    fn chain(self, system: SystemB) -> ChainSystem<SystemA::System, SystemB::System> {
        let system_a = IntoSystem::into_system(self);
        let system_b = IntoSystem::into_system(system);
        ChainSystem {
            name: Cow::Owned(format!("Chain({}, {})", system_a.name(), system_b.name())),
            system_a,
            system_b,
            archetype_component_access: Default::default(),
            component_access: Default::default(),
        }
    }
}

/// A collection of common adapters for [chaining](super::ChainSystem) the result of a system.
pub mod adapter {
    use crate::system::In;
    use std::fmt::Debug;

    /// Converts a regular function into a system adapter.
    ///
    /// # Examples
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// return1
    ///     .chain(system_adapter::new(u32::try_from))
    ///     .chain(system_adapter::unwrap)
    ///     .chain(print);
    ///
    /// fn return1() -> u64 { 1 }
    /// fn print(In(x): In<impl std::fmt::Debug>) {
    ///     println!("{x:?}");
    /// }
    /// ```
    pub fn new<T, U>(mut f: impl FnMut(T) -> U) -> impl FnMut(In<T>) -> U {
        move |In(x)| f(x)
    }

    /// System adapter that unwraps the `Ok` variant of a [`Result`].
    /// This is useful for fallible systems that should panic in the case of an error.
    ///
    /// There is no equivalent adapter for [`Option`]. Instead, it's best to provide
    /// an error message and convert to a `Result` using `ok_or{_else}`.
    ///
    /// # Examples
    ///
    /// Panicking on error
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(StageLabel)]
    /// # enum CoreStage { Update };
    ///
    /// // Building a new schedule/app...
    /// # use bevy_ecs::schedule::SystemStage;
    /// # let mut sched = Schedule::default(); sched
    /// #     .add_stage(CoreStage::Update, SystemStage::single_threaded())
    ///     .add_system_to_stage(
    ///         CoreStage::Update,
    ///         // Panic if the load system returns an error.
    ///         load_save_system.chain(system_adapter::unwrap)
    ///     )
    ///     // ...
    /// #   ;
    /// # let mut world = World::new();
    /// # sched.run(&mut world);
    ///
    /// // A system which may fail irreparably.
    /// fn load_save_system() -> Result<(), std::io::Error> {
    ///     let save_file = open_file("my_save.json")?;
    ///     dbg!(save_file);
    ///     Ok(())
    /// }
    /// # fn open_file(name: &str) -> Result<&'static str, std::io::Error>
    /// # { Ok("hello world") }
    /// ```
    pub fn unwrap<T, E: Debug>(In(res): In<Result<T, E>>) -> T {
        res.unwrap()
    }

    /// System adapter that ignores the output of the previous system in a chain.
    /// This is useful for fallible systems that should simply return early in case of an `Err`/`None`.
    ///
    /// # Examples
    ///
    /// Returning early
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// // Marker component for an enemy entity.
    /// #[derive(Component)]
    /// struct Monster;
    /// #
    /// # #[derive(StageLabel)]
    /// # enum CoreStage { Update };
    ///
    /// // Building a new schedule/app...
    /// # use bevy_ecs::schedule::SystemStage;
    /// # let mut sched = Schedule::default(); sched
    /// #     .add_stage(CoreStage::Update, SystemStage::single_threaded())
    ///     .add_system_to_stage(
    ///         CoreStage::Update,
    ///         // If the system fails, just move on and try again next frame.
    ///         fallible_system.chain(system_adapter::ignore)
    ///     )
    ///     // ...
    /// #   ;
    /// # let mut world = World::new();
    /// # sched.run(&mut world);
    ///
    /// // A system which may return early. It's more convenient to use the `?` operator for this.
    /// fn fallible_system(
    ///     q: Query<Entity, With<Monster>>
    /// ) -> Option<()> {
    ///     let monster_id = q.iter().next()?;
    ///     println!("Monster entity is {monster_id:?}");
    ///     Some(())
    /// }
    /// ```
    pub fn ignore<T>(In(_): In<T>) {}

    #[cfg(test)]
    #[test]
    fn assert_systems() {
        use std::str::FromStr;

        use crate::{prelude::*, system::assert_is_system};

        /// Mocks a system that returns a value of type `T`.
        fn returning<T>() -> T {
            unimplemented!()
        }

        assert_is_system(returning::<Result<u32, std::io::Error>>.chain(unwrap));
        assert_is_system(returning::<Option<()>>.chain(ignore));
        assert_is_system(returning::<&str>.chain(new(u64::from_str)).chain(unwrap));
    }
}
