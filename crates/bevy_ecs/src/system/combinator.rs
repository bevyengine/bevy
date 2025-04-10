use alloc::{borrow::Cow, format, vec::Vec};
use core::marker::PhantomData;

use crate::{
    archetype::ArchetypeComponentId,
    component::{ComponentId, Tick},
    prelude::World,
    query::Access,
    schedule::InternedSystemSet,
    system::{input::SystemInput, SystemIn, SystemParamValidationError},
    world::unsafe_world_cell::UnsafeWorldCell,
};

use super::{IntoSystem, ReadOnlySystem, System};

/// Customizes the behavior of a [`CombinatorSystem`].
///
/// # Examples
///
/// ```
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::system::{CombinatorSystem, Combine};
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
///         a: impl FnOnce(A::In) -> A::Out,
///         b: impl FnOnce(B::In) -> B::Out,
///     ) -> Self::Out {
///         a(()) ^ b(())
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
///     std::borrow::Cow::Borrowed("a ^ b"),
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
        a: impl FnOnce(SystemIn<'_, A>) -> A::Out,
        b: impl FnOnce(SystemIn<'_, B>) -> B::Out,
    ) -> Self::Out;
}

/// A [`System`] defined by combining two other systems.
/// The behavior of this combinator is specified by implementing the [`Combine`] trait.
/// For a full usage example, see the docs for [`Combine`].
pub struct CombinatorSystem<Func, A, B> {
    _marker: PhantomData<fn() -> Func>,
    a: A,
    b: B,
    name: Cow<'static, str>,
    component_access: Access<ComponentId>,
    archetype_component_access: Access<ArchetypeComponentId>,
}

impl<Func, A, B> CombinatorSystem<Func, A, B> {
    /// Creates a new system that combines two inner systems.
    ///
    /// The returned system will only be usable if `Func` implements [`Combine<A, B>`].
    pub const fn new(a: A, b: B, name: Cow<'static, str>) -> Self {
        Self {
            _marker: PhantomData,
            a,
            b,
            name,
            component_access: Access::new(),
            archetype_component_access: Access::new(),
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

    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn component_access(&self) -> &Access<ComponentId> {
        &self.component_access
    }

    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        &self.archetype_component_access
    }

    fn is_send(&self) -> bool {
        self.a.is_send() && self.b.is_send()
    }

    fn is_exclusive(&self) -> bool {
        self.a.is_exclusive() || self.b.is_exclusive()
    }

    fn has_deferred(&self) -> bool {
        self.a.has_deferred() || self.b.has_deferred()
    }

    unsafe fn run_unsafe(
        &mut self,
        input: SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Self::Out {
        Func::combine(
            input,
            // SAFETY: The world accesses for both underlying systems have been registered,
            // so the caller will guarantee that no other systems will conflict with `a` or `b`.
            // If either system has `is_exclusive()`, then the combined system also has `is_exclusive`.
            // Since these closures are `!Send + !Sync + !'static`, they can never be called
            // in parallel, so their world accesses will not conflict with each other.
            // Additionally, `update_archetype_component_access` has been called,
            // which forwards to the implementations for `self.a` and `self.b`.
            |input| unsafe { self.a.run_unsafe(input, world) },
            // SAFETY: See the comment above.
            |input| unsafe { self.b.run_unsafe(input, world) },
        )
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
        // SAFETY: Delegate to other `System` implementations.
        unsafe { self.a.validate_param_unsafe(world) }
    }

    fn initialize(&mut self, world: &mut World) {
        self.a.initialize(world);
        self.b.initialize(world);
        self.component_access.extend(self.a.component_access());
        self.component_access.extend(self.b.component_access());
    }

    fn update_archetype_component_access(&mut self, world: UnsafeWorldCell) {
        self.a.update_archetype_component_access(world);
        self.b.update_archetype_component_access(world);

        self.archetype_component_access
            .extend(self.a.archetype_component_access());
        self.archetype_component_access
            .extend(self.b.archetype_component_access());
    }

    fn check_change_tick(&mut self, change_tick: Tick) {
        self.a.check_change_tick(change_tick);
        self.b.check_change_tick(change_tick);
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
        PipeSystem::new(system_a, system_b, Cow::Owned(name))
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
pub struct PipeSystem<A, B> {
    a: A,
    b: B,
    name: Cow<'static, str>,
    component_access: Access<ComponentId>,
    archetype_component_access: Access<ArchetypeComponentId>,
}

impl<A, B> PipeSystem<A, B>
where
    A: System,
    B: System,
    for<'a> B::In: SystemInput<Inner<'a> = A::Out>,
{
    /// Creates a new system that pipes two inner systems.
    pub const fn new(a: A, b: B, name: Cow<'static, str>) -> Self {
        Self {
            a,
            b,
            name,
            component_access: Access::new(),
            archetype_component_access: Access::new(),
        }
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

    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn component_access(&self) -> &Access<ComponentId> {
        &self.component_access
    }

    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        &self.archetype_component_access
    }

    fn is_send(&self) -> bool {
        self.a.is_send() && self.b.is_send()
    }

    fn is_exclusive(&self) -> bool {
        self.a.is_exclusive() || self.b.is_exclusive()
    }

    fn has_deferred(&self) -> bool {
        self.a.has_deferred() || self.b.has_deferred()
    }

    unsafe fn run_unsafe(
        &mut self,
        input: SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Self::Out {
        let value = self.a.run_unsafe(input, world);
        self.b.run_unsafe(value, world)
    }

    fn apply_deferred(&mut self, world: &mut World) {
        self.a.apply_deferred(world);
        self.b.apply_deferred(world);
    }

    fn queue_deferred(&mut self, mut world: crate::world::DeferredWorld) {
        self.a.queue_deferred(world.reborrow());
        self.b.queue_deferred(world);
    }

    /// This method uses "early out" logic: if the first system fails validation,
    /// the second system is not validated.
    ///
    /// Because the system validation is performed upfront, this can lead to situations
    /// where later systems pass validation, but fail at runtime due to changes made earlier
    /// in the piped systems.
    // TODO: ensure that systems are only validated just before they are run.
    // Fixing this will require fundamentally rethinking how piped systems work:
    // they're currently treated as a single system from the perspective of the scheduler.
    // See https://github.com/bevyengine/bevy/issues/18796
    unsafe fn validate_param_unsafe(
        &mut self,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // SAFETY: Delegate to the `System` implementation for `a`.
        unsafe { self.a.validate_param_unsafe(world) }?;

        // SAFETY: Delegate to the `System` implementation for `b`.
        unsafe { self.b.validate_param_unsafe(world) }?;

        Ok(())
    }

    fn initialize(&mut self, world: &mut World) {
        self.a.initialize(world);
        self.b.initialize(world);
        self.component_access.extend(self.a.component_access());
        self.component_access.extend(self.b.component_access());
    }

    fn update_archetype_component_access(&mut self, world: UnsafeWorldCell) {
        self.a.update_archetype_component_access(world);
        self.b.update_archetype_component_access(world);

        self.archetype_component_access
            .extend(self.a.archetype_component_access());
        self.archetype_component_access
            .extend(self.b.archetype_component_access());
    }

    fn check_change_tick(&mut self, change_tick: Tick) {
        self.a.check_change_tick(change_tick);
        self.b.check_change_tick(change_tick);
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
