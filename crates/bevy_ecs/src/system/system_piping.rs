use crate::system::{IntoSystem, System};
use std::borrow::Cow;

use super::{CombinatorSystem, Combine};

/// A [`System`] created by piping the output of the first system into the input of the second.
///
/// This can be repeated indefinitely, but system pipes cannot branch: the output is consumed by the receiving system.
///
/// Given two systems `A` and `B`, A may be piped into `B` as `A.pipe(B)` if the output type of `A` is
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
///     // pipe the `parse_message_system`'s output into the `filter_system`s input
///     let mut piped_system = parse_message_system.pipe(filter_system);
///     piped_system.initialize(&mut world);
///     assert_eq!(piped_system.run((), &mut world), Some(42));
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
pub type PipeSystem<SystemA, SystemB> = CombinatorSystem<Pipe, SystemA, SystemB>;

#[doc(hidden)]
pub struct Pipe;

impl<A, B> Combine<A, B> for Pipe
where
    A: System,
    B: System<In = A::Out>,
{
    type In = A::In;
    type Out = B::Out;

    fn combine(
        input: Self::In,
        a: impl FnOnce(<A as System>::In) -> <A as System>::Out,
        b: impl FnOnce(<B as System>::In) -> <B as System>::Out,
    ) -> Self::Out {
        let value = a(input);
        b(value)
    }
}

/// An extension trait providing the [`IntoPipeSystem::pipe`] method to pass input from one system into the next.
///
/// The first system must have return type `T`
/// and the second system must have [`In<T>`](crate::system::In) as its first system parameter.
///
/// This trait is blanket implemented for all system pairs that fulfill the type requirements.
///
/// See [`PipeSystem`].
pub trait IntoPipeSystem<ParamA, Payload, SystemB, ParamB, Out>:
    IntoSystem<(), Payload, ParamA> + Sized
where
    SystemB: IntoSystem<Payload, Out, ParamB>,
{
    /// Pass the output of this system `A` into a second system `B`, creating a new compound system.
    fn pipe(self, system: SystemB) -> PipeSystem<Self::System, SystemB::System>;
}

impl<SystemA, ParamA, Payload, SystemB, ParamB, Out>
    IntoPipeSystem<ParamA, Payload, SystemB, ParamB, Out> for SystemA
where
    SystemA: IntoSystem<(), Payload, ParamA>,
    SystemB: IntoSystem<Payload, Out, ParamB>,
{
    fn pipe(self, system: SystemB) -> PipeSystem<SystemA::System, SystemB::System> {
        let system_a = IntoSystem::into_system(self);
        let system_b = IntoSystem::into_system(system);
        let name = format!("Pipe({}, {})", system_a.name(), system_b.name());
        PipeSystem::new(system_a, system_b, Cow::Owned(name))
    }
}

/// A collection of common adapters for [piping](super::PipeSystem) the result of a system.
pub mod adapter {
    use crate::system::In;
    use bevy_utils::tracing;
    use std::fmt::Debug;

    /// Converts a regular function into a system adapter.
    ///
    /// # Examples
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// fn return1() -> u64 { 1 }
    ///
    /// return1
    ///     .pipe(system_adapter::new(u32::try_from))
    ///     .pipe(system_adapter::unwrap)
    ///     .pipe(print);
    ///
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
    ///
    /// // Building a new schedule/app...
    /// let mut sched = Schedule::default();
    /// sched.add_system(
    ///         // Panic if the load system returns an error.
    ///         load_save_system.pipe(system_adapter::unwrap)
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

    /// System adapter that utilizes the [`bevy_utils::tracing::info!`] macro to print system information.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// // Building a new schedule/app...
    /// let mut sched = Schedule::default();
    /// sched.add_system(
    ///         // Prints system information.
    ///         data_pipe_system.pipe(system_adapter::info)
    ///     )
    ///     // ...
    /// #   ;
    /// # let mut world = World::new();
    /// # sched.run(&mut world);
    ///
    /// // A system that returns a String output.
    /// fn data_pipe_system() -> String {
    ///     "42".to_string()
    /// }
    /// ```
    pub fn info<T: Debug>(In(data): In<T>) {
        tracing::info!("{:?}", data);
    }

    /// System adapter that utilizes the [`bevy_utils::tracing::debug!`] macro to print the output of a system.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// // Building a new schedule/app...
    /// let mut sched = Schedule::default();
    /// sched.add_system(
    ///         // Prints debug data from system.
    ///         parse_message_system.pipe(system_adapter::dbg)
    ///     )
    ///     // ...
    /// #   ;
    /// # let mut world = World::new();
    /// # sched.run(&mut world);
    ///
    /// // A system that returns a Result<usize, String> output.
    /// fn parse_message_system() -> Result<usize, std::num::ParseIntError> {
    ///     Ok("42".parse()?)
    /// }
    /// ```
    pub fn dbg<T: Debug>(In(data): In<T>) {
        tracing::debug!("{:?}", data);
    }

    /// System adapter that utilizes the [`bevy_utils::tracing::warn!`] macro to print the output of a system.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// // Building a new schedule/app...
    /// # let mut sched = Schedule::default();
    /// sched.add_system(
    ///         // Prints system warning if system returns an error.
    ///         warning_pipe_system.pipe(system_adapter::warn)
    ///     )
    ///     // ...
    /// #   ;
    /// # let mut world = World::new();
    /// # sched.run(&mut world);
    ///
    /// // A system that returns a Result<(), String> output.
    /// fn warning_pipe_system() -> Result<(), String> {
    ///     Err("Got to rusty?".to_string())
    /// }
    /// ```
    pub fn warn<E: Debug>(In(res): In<Result<(), E>>) {
        if let Err(warn) = res {
            tracing::warn!("{:?}", warn);
        }
    }

    /// System adapter that utilizes the [`bevy_utils::tracing::error!`] macro to print the output of a system.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    /// // Building a new schedule/app...
    /// let mut sched = Schedule::default();
    /// sched.add_system(
    ///         // Prints system error if system fails.
    ///         parse_error_message_system.pipe(system_adapter::error)
    ///     )
    ///     // ...
    /// #   ;
    /// # let mut world = World::new();
    /// # sched.run(&mut world);
    ///
    /// // A system that returns a Result<())> output.
    /// fn parse_error_message_system() -> Result<(), String> {
    ///    Err("Some error".to_owned())
    /// }
    /// ```
    pub fn error<E: Debug>(In(res): In<Result<(), E>>) {
        if let Err(error) = res {
            tracing::error!("{:?}", error);
        }
    }

    /// System adapter that ignores the output of the previous system in a pipe.
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
    ///
    /// // Building a new schedule/app...
    /// # let mut sched = Schedule::default(); sched
    ///     .add_system(
    ///         // If the system fails, just move on and try again next frame.
    ///         fallible_system.pipe(system_adapter::ignore)
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

        /// Mocks an exclusive system that takes an input and returns an output.
        fn exclusive_in_out<A, B>(_: In<A>, _: &mut World) -> B {
            unimplemented!()
        }

        fn not(In(val): In<bool>) -> bool {
            !val
        }

        assert_is_system(returning::<Result<u32, std::io::Error>>.pipe(unwrap));
        assert_is_system(returning::<Option<()>>.pipe(ignore));
        assert_is_system(returning::<&str>.pipe(new(u64::from_str)).pipe(unwrap));
        assert_is_system(exclusive_in_out::<(), Result<(), std::io::Error>>.pipe(error));
        assert_is_system(returning::<bool>.pipe(exclusive_in_out::<bool, ()>));

        returning::<()>.run_if(returning::<bool>.pipe(not));
    }
}
