use std::{borrow::Cow, cell::UnsafeCell, marker::PhantomData};

use bevy_ptr::UnsafeCellDeref;

use crate::{
    archetype::ArchetypeComponentId,
    component::{ComponentId, Tick},
    prelude::World,
    query::Access,
    world::unsafe_world_cell::UnsafeWorldCell,
};

use super::{ReadOnlySystem, System};

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
pub trait Combine<A: System, B: System> {
    /// The [input](System::In) type for a [`CombinatorSystem`].
    type In;

    /// The [output](System::Out) type for a [`CombinatorSystem`].
    type Out;

    /// When used in a [`CombinatorSystem`], this function customizes how
    /// the two composite systems are invoked and their outputs are combined.
    ///
    /// See the trait-level docs for [`Combine`] for an example implementation.
    fn combine(
        input: Self::In,
        a: impl FnOnce(A::In) -> A::Out,
        b: impl FnOnce(B::In) -> B::Out,
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

    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
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

    unsafe fn run_unsafe(&mut self, input: Self::In, world: UnsafeWorldCell) -> Self::Out {
        Func::combine(
            input,
            // SAFETY: The world accesses for both underlying systems have been registered,
            // so the caller will guarantee that no other systems will conflict with `a` or `b`.
            // Since these closures are `!Send + !Sync + !'static`, they can never be called
            // in parallel, so their world accesses will not conflict with each other.
            // Additionally, `update_archetype_component_access` has been called,
            // which forwards to the implementations for `self.a` and `self.b`.
            |input| self.a.run_unsafe(input, world),
            |input| self.b.run_unsafe(input, world),
        )
    }

    fn run<'w>(&mut self, input: Self::In, world: &'w mut World) -> Self::Out {
        // SAFETY: Converting `&mut T` -> `&UnsafeCell<T>`
        // is explicitly allowed in the docs for `UnsafeCell`.
        let world: &'w UnsafeCell<World> = unsafe { std::mem::transmute(world) };
        Func::combine(
            input,
            // SAFETY: Since these closures are `!Send + !Sync + !'static`, they can never
            // be called in parallel. Since mutable access to `world` only exists within
            // the scope of either closure, we can be sure they will never alias one another.
            |input| self.a.run(input, unsafe { world.deref_mut() }),
            #[allow(clippy::undocumented_unsafe_blocks)]
            |input| self.b.run(input, unsafe { world.deref_mut() }),
        )
    }

    fn apply_deferred(&mut self, world: &mut World) {
        self.a.apply_deferred(world);
        self.b.apply_deferred(world);
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

    fn get_last_run(&self) -> Tick {
        self.a.get_last_run()
    }

    fn set_last_run(&mut self, last_run: Tick) {
        self.a.set_last_run(last_run);
        self.b.set_last_run(last_run);
    }

    fn default_system_sets(&self) -> Vec<Box<dyn crate::schedule::SystemSet>> {
        let mut default_sets = self.a.default_system_sets();
        default_sets.append(&mut self.b.default_system_sets());
        default_sets
    }
}

/// SAFETY: Both systems are read-only, so any system created by combining them will only read from the world.
unsafe impl<A, B, Func> ReadOnlySystem for CombinatorSystem<Func, A, B>
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
        a: impl FnOnce(A::In) -> A::Out,
        b: impl FnOnce(B::In) -> B::Out,
    ) -> Self::Out {
        let value = a(input);
        b(value)
    }
}
