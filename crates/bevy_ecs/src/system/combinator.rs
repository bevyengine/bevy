use alloc::{format, vec::Vec};
use bevy_utils::prelude::DebugName;
use core::marker::PhantomData;

use crate::{
    component::{CheckChangeTicks, Tick},
    prelude::World,
    query::FilteredAccessSet,
    schedule::InternedSystemSet,
    system::{input::SystemInput, SystemIn, SystemParamValidationError},
    world::unsafe_world_cell::UnsafeWorldCell,
};

use super::{IntoSystem, ReadOnlySystem, RunSystemError, System};

/// Customizes the behavior of a [`CombinatorSystem`].
///
/// # Examples
///
/// ```
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::system::{CombinatorSystem, Combine, RunSystemError};
///
/// // A system combinator that performs an exclusive-or (XOR)
/// // operation on the output of two systems.
/// pub type Xor<A, B> = CombinatorSystem<XorMarker, A, B>;
///
/// // This struct is used to customize the behavior of our combinator.
/// pub struct XorMarker;
///
/// impl<A, B> Combine<A, B> for XorMarker
/// where
///     A: System<In = (), Out = bool>,
///     B: System<In = (), Out = bool>,
/// {
///     type In = ();
///     type Out = bool;
///
///     fn combine(
///         _input: Self::In,
///         a: impl FnOnce(A::In) -> Result<A::Out, RunSystemError>,
///         b: impl FnOnce(B::In) -> Result<B::Out, RunSystemError>,
///     ) -> Result<Self::Out, RunSystemError> {
///         Ok(a(())? ^ b(())?)
///     }
/// }
///
/// # #[derive(Resource, PartialEq, Eq)] struct A(u32);
/// # #[derive(Resource, PartialEq, Eq)] struct B(u32);
/// # #[derive(Resource, Default)] struct RanFlag(bool);
/// # let mut world = World::new();
/// # world.init_resource::<RanFlag>();
/// #
/// # let mut app = Schedule::default();
/// app.add_systems(my_system.run_if(Xor::new(
///     IntoSystem::into_system(resource_equals(A(1))),
///     IntoSystem::into_system(resource_equals(B(1))),
///     // The name of the combined system.
///     "a ^ b".into(),
/// )));
/// # fn my_system(mut flag: ResMut<RanFlag>) { flag.0 = true; }
/// #
/// # world.insert_resource(A(0));
/// # world.insert_resource(B(0));
/// # app.run(&mut world);
/// # // Neither condition passes, so the system does not run.
/// # assert!(!world.resource::<RanFlag>().0);
/// #
/// # world.insert_resource(A(1));
/// # app.run(&mut world);
/// # // Only the first condition passes, so the system runs.
/// # assert!(world.resource::<RanFlag>().0);
/// # world.resource_mut::<RanFlag>().0 = false;
/// #
/// # world.insert_resource(B(1));
/// # app.run(&mut world);
/// # // Both conditions pass, so the system does not run.
/// # assert!(!world.resource::<RanFlag>().0);
/// #
/// # world.insert_resource(A(0));
/// # app.run(&mut world);
/// # // Only the second condition passes, so the system runs.
/// # assert!(world.resource::<RanFlag>().0);
/// # world.resource_mut::<RanFlag>().0 = false;
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` can not combine systems `{A}` and `{B}`",
    label = "invalid system combination",
    note = "the inputs and outputs of `{A}` and `{B}` are not compatible with this combiner"
)]
pub trait Combine<A: System, B: System> {
    /// The [input](System::In) type for a [`CombinatorSystem`].
    type In: SystemInput;

    /// The [output](System::Out) type for a [`CombinatorSystem`].
    type Out;

    /// When used in a [`CombinatorSystem`], this function customizes how
    /// the two composite systems are invoked and their outputs are combined.
    ///
    /// See the trait-level docs for [`Combine`] for an example implementation.
    fn combine(
        input: <Self::In as SystemInput>::Inner<'_>,
        a: impl FnOnce(SystemIn<'_, A>) -> Result<A::Out, RunSystemError>,
        b: impl FnOnce(SystemIn<'_, B>) -> Result<B::Out, RunSystemError>,
    ) -> Result<Self::Out, RunSystemError>;
}

/// A [`System`] defined by combining two other systems.
/// The behavior of this combinator is specified by implementing the [`Combine`] trait.
/// For a full usage example, see the docs for [`Combine`].
pub struct CombinatorSystem<Func, A, B> {
    _marker: PhantomData<fn() -> Func>,
    a: A,
    b: B,
    name: DebugName,
}

impl<Func, A, B> CombinatorSystem<Func, A, B> {
    /// Creates a new system that combines two inner systems.
    ///
    /// The returned system will only be usable if `Func` implements [`Combine<A, B>`].
    pub fn new(a: A, b: B, name: DebugName) -> Self {
        Self {
            _marker: PhantomData,
            a,
            b,
            name,
        }
    }
}

impl<A, B, Func> System for CombinatorSystem<Func, A, B>
where
    Func: Combine<A, B> + 'static,
    A: System,
    B: System,
{
    type In = Func::In;
    type Out = Func::Out;

    fn name(&self) -> DebugName {
        self.name.clone()
    }

    #[inline]
    fn flags(&self) -> super::SystemStateFlags {
        self.a.flags() | self.b.flags()
    }

    unsafe fn run_unsafe(
        &mut self,
        input: SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Result<Self::Out, RunSystemError> {
        Func::combine(
            input,
            // SAFETY: The world accesses for both underlying systems have been registered,
            // so the caller will guarantee that no other systems will conflict with `a` or `b`.
            // If either system has `is_exclusive()`, then the combined system also has `is_exclusive`.
            // Since these closures are `!Send + !Sync + !'static`, they can never be called
            // in parallel, so their world accesses will not conflict with each other.
            |input| unsafe { self.a.run_unsafe(input, world) },
            // `Self::validate_param_unsafe` already validated the first system,
            // but we still need to validate the second system once the first one runs.
            // SAFETY: See the comment above.
            |input| unsafe {
                self.b.validate_param_unsafe(world)?;
                self.b.run_unsafe(input, world)
            },
        )
    }

    #[cfg(feature = "hotpatching")]
    #[inline]
    fn refresh_hotpatch(&mut self) {
        self.a.refresh_hotpatch();
        self.b.refresh_hotpatch();
    }

    #[inline]
    fn apply_deferred(&mut self, world: &mut World) {
        self.a.apply_deferred(world);
        self.b.apply_deferred(world);
    }

    #[inline]
    fn queue_deferred(&mut self, mut world: crate::world::DeferredWorld) {
        self.a.queue_deferred(world.reborrow());
        self.b.queue_deferred(world);
    }

    #[inline]
    unsafe fn validate_param_unsafe(
        &mut self,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // We only validate parameters for the first system,
        // since it may make changes to the world that affect
        // whether the second system has valid parameters.
        // The second system will be validated in `Self::run_unsafe`.
        // SAFETY: Delegate to other `System` implementations.
        unsafe { self.a.validate_param_unsafe(world) }
    }

    fn initialize(&mut self, world: &mut World) -> FilteredAccessSet {
        let mut a_access = self.a.initialize(world);
        let b_access = self.b.initialize(world);
        a_access.extend(b_access);
        a_access
    }

    fn check_change_tick(&mut self, check: CheckChangeTicks) {
        self.a.check_change_tick(check);
        self.b.check_change_tick(check);
    }

    fn default_system_sets(&self) -> Vec<InternedSystemSet> {
        let mut default_sets = self.a.default_system_sets();
        default_sets.append(&mut self.b.default_system_sets());
        default_sets
    }

    fn get_last_run(&self) -> Tick {
        self.a.get_last_run()
    }

    fn set_last_run(&mut self, last_run: Tick) {
        self.a.set_last_run(last_run);
        self.b.set_last_run(last_run);
    }
}

/// SAFETY: Both systems are read-only, so any system created by combining them will only read from the world.
unsafe impl<Func, A, B> ReadOnlySystem for CombinatorSystem<Func, A, B>
where
    Func: Combine<A, B> + 'static,
    A: ReadOnlySystem,
    B: ReadOnlySystem,
{
}

impl<Func, A, B> Clone for CombinatorSystem<Func, A, B>
where
    A: Clone,
    B: Clone,
{
    /// Clone the combined system. The cloned instance must be `.initialize()`d before it can run.
    fn clone(&self) -> Self {
        CombinatorSystem::new(self.a.clone(), self.b.clone(), self.name.clone())
    }
}

/// An [`IntoSystem`] creating an instance of [`PipeSystem`].
#[derive(Clone)]
pub struct IntoPipeSystem<A, B> {
    a: A,
    b: B,
}

impl<A, B> IntoPipeSystem<A, B> {
    /// Creates a new [`IntoSystem`] that pipes two inner systems.
    pub const fn new(a: A, b: B) -> Self {
        Self { a, b }
    }
}

#[doc(hidden)]
pub struct IsPipeSystemMarker;

impl<A, B, IA, OA, IB, OB, MA, MB> IntoSystem<IA, OB, (IsPipeSystemMarker, OA, IB, MA, MB)>
    for IntoPipeSystem<A, B>
where
    IA: SystemInput,
    A: IntoSystem<IA, OA, MA>,
    B: IntoSystem<IB, OB, MB>,
    for<'a> IB: SystemInput<Inner<'a> = OA>,
{
    type System = PipeSystem<A::System, B::System>;

    fn into_system(this: Self) -> Self::System {
        let system_a = IntoSystem::into_system(this.a);
        let system_b = IntoSystem::into_system(this.b);
        let name = format!("Pipe({}, {})", system_a.name(), system_b.name());
        PipeSystem::new(system_a, system_b, DebugName::owned(name))
    }
}

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
///     let mut piped_system = IntoSystem::into_system(parse_message_system.pipe(filter_system));
///     piped_system.initialize(&mut world);
///     assert_eq!(piped_system.run((), &mut world).unwrap(), Some(42));
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
pub struct PipeSystem<A, B> {
    a: A,
    b: B,
    name: DebugName,
}

impl<A, B> PipeSystem<A, B>
where
    A: System,
    B: System,
    for<'a> B::In: SystemInput<Inner<'a> = A::Out>,
{
    /// Creates a new system that pipes two inner systems.
    pub fn new(a: A, b: B, name: DebugName) -> Self {
        Self { a, b, name }
    }
}

impl<A, B> System for PipeSystem<A, B>
where
    A: System,
    B: System,
    for<'a> B::In: SystemInput<Inner<'a> = A::Out>,
{
    type In = A::In;
    type Out = B::Out;

    fn name(&self) -> DebugName {
        self.name.clone()
    }

    #[inline]
    fn flags(&self) -> super::SystemStateFlags {
        self.a.flags() | self.b.flags()
    }

    unsafe fn run_unsafe(
        &mut self,
        input: SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Result<Self::Out, RunSystemError> {
        let value = self.a.run_unsafe(input, world)?;
        // `Self::validate_param_unsafe` already validated the first system,
        // but we still need to validate the second system once the first one runs.
        self.b.validate_param_unsafe(world)?;
        self.b.run_unsafe(value, world)
    }

    #[cfg(feature = "hotpatching")]
    #[inline]
    fn refresh_hotpatch(&mut self) {
        self.a.refresh_hotpatch();
        self.b.refresh_hotpatch();
    }

    fn apply_deferred(&mut self, world: &mut World) {
        self.a.apply_deferred(world);
        self.b.apply_deferred(world);
    }

    fn queue_deferred(&mut self, mut world: crate::world::DeferredWorld) {
        self.a.queue_deferred(world.reborrow());
        self.b.queue_deferred(world);
    }

    unsafe fn validate_param_unsafe(
        &mut self,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // We only validate parameters for the first system,
        // since it may make changes to the world that affect
        // whether the second system has valid parameters.
        // The second system will be validated in `Self::run_unsafe`.
        // SAFETY: Delegate to the `System` implementation for `a`.
        unsafe { self.a.validate_param_unsafe(world) }
    }

    fn initialize(&mut self, world: &mut World) -> FilteredAccessSet {
        let mut a_access = self.a.initialize(world);
        let b_access = self.b.initialize(world);
        a_access.extend(b_access);
        a_access
    }

    fn check_change_tick(&mut self, check: CheckChangeTicks) {
        self.a.check_change_tick(check);
        self.b.check_change_tick(check);
    }

    fn default_system_sets(&self) -> Vec<InternedSystemSet> {
        let mut default_sets = self.a.default_system_sets();
        default_sets.append(&mut self.b.default_system_sets());
        default_sets
    }

    fn get_last_run(&self) -> Tick {
        self.a.get_last_run()
    }

    fn set_last_run(&mut self, last_run: Tick) {
        self.a.set_last_run(last_run);
        self.b.set_last_run(last_run);
    }
}

/// SAFETY: Both systems are read-only, so any system created by piping them will only read from the world.
unsafe impl<A, B> ReadOnlySystem for PipeSystem<A, B>
where
    A: ReadOnlySystem,
    B: ReadOnlySystem,
    for<'a> B::In: SystemInput<Inner<'a> = A::Out>,
{
}

#[cfg(test)]
mod tests {

    #[test]
    fn exclusive_system_piping_is_possible() {
        use crate::prelude::*;

        fn my_exclusive_system(_world: &mut World) -> u32 {
            1
        }

        fn out_pipe(input: In<u32>) {
            assert!(input.0 == 1);
        }

        let mut world = World::new();

        let mut schedule = Schedule::default();
        schedule.add_systems(my_exclusive_system.pipe(out_pipe));

        schedule.run(&mut world);
    }
}
