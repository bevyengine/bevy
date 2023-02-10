use std::{borrow::Cow, marker::PhantomData};

use crate::{
    archetype::ArchetypeComponentId, component::ComponentId, prelude::World, query::Access,
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
/// pub type Xor<A, B> = CombinatorSystem<A, B, XorMarker>;
///
/// // This struct is used to customize the behavior of our combinator.
/// pub struct XorMarker;
///
/// impl<A, B> Combine<A, B> for XorMarker
///     where A: System<In = (), Out = bool>,
///     where B: System<In = (), Out = bool>,
/// {
///     type In = ();
///     type Out = bool;
///
///     fn combine(
///         _input: Self::In,
///         world: &World,
///         a: impl FnOnce(A::In, &World) -> A::Out,
///         b: impl FnOnce(B::In, &World) -> B::Out,
///     ) -> Self::Out {
///         a((), world) ^ b((), world)
///     }
///
///     fn combine_exclusive(
///         _input: Self::In,
///         world: &mut World,
///         a: impl FnOnce(A::In, &mut World) -> A::Out,
///         b: impl FnOnce(B::In, &mut World) -> B::Out,
///     ) -> Self::Out {
///         a((), world) ^ b((), world)
///     }
/// }
///
/// # #[derive(Resource)] struct A(u32);
/// # #[derive(Resource)] struct B(u32);
/// # #[derive(Resource, Default)] struct RanFlag(bool);
/// # let mut world = World::new();
/// # world.init_resource::<RanFlag>();
/// #
/// # let mut app = Schedule::new();
/// app.add_system(my_system.run_if(Xor::new(
///     IntoSystem::into_system(state_equals(A(1))),
///     IntoSystem::into_system(state_equals(B(1))),
///     // The name of the combined system.
///     Cow::Borrowed("a ^ b"),
/// )));
/// # fn my_system(mut flag: ResMut<RanFlag>) { flag.0 = true; }
/// #
/// # world.insert_resource(A(0));
/// # world.insert_resource(B(0));
/// # schedule.run(&mut world);
/// # // Neither condition passes, so the system does not run.
/// # assert!(!world.resource::<RanFlag>().0);
/// #
/// # world.insert_resource(A(1));
/// # schedule.run(&mut world);
/// # // Only the first condition passes, so the system runs.
/// # assert!(world.resource::<RanFlag>().0);
/// # world.resource_mut::<RanFlag>().0 = false;
/// #
/// # world.insert_resource(B(1));
/// # schedule.run(&mut world);
/// # // Both conditions pass, so the system does not run.
/// # assert!(!world.resource::<RanFlag>().0);
/// #
/// # world.insert_resource(A(0));
/// # schedule.run(&mut world);
/// # // Only the second condition passes, so the system runs.
/// # assert!(world.resource::<RanFlag>().0);
/// # world.resource_mut::<RanFlag>().0 = false;
/// ```
pub trait Combine<A: System, B: System> {
    type In;
    type Out;

    fn combine(
        input: Self::In,
        world: &World,
        a: impl FnOnce(A::In, &World) -> A::Out,
        b: impl FnOnce(B::In, &World) -> B::Out,
    ) -> Self::Out;

    fn combine_exclusive(
        input: Self::In,
        world: &mut World,
        a: impl FnOnce(A::In, &mut World) -> A::Out,
        b: impl FnOnce(B::In, &mut World) -> B::Out,
    ) -> Self::Out;
}

/// A [`System`] defined by combining two other systems.
/// The behavior of this combinator is specified by implementing the [`Combine`] trait.
pub struct CombinatorSystem<Func, A, B> {
    _marker: PhantomData<fn() -> Func>,
    a: A,
    b: B,
    name: Cow<'static, str>,
    component_access: Access<ComponentId>,
    archetype_component_access: Access<ArchetypeComponentId>,
}

impl<Func, A, B> CombinatorSystem<Func, A, B> {
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

    fn component_access(&self) -> &crate::query::Access<crate::component::ComponentId> {
        &self.component_access
    }

    fn archetype_component_access(
        &self,
    ) -> &crate::query::Access<crate::archetype::ArchetypeComponentId> {
        &self.archetype_component_access
    }

    fn is_send(&self) -> bool {
        self.a.is_send() && self.b.is_send()
    }

    fn is_exclusive(&self) -> bool {
        self.a.is_exclusive() || self.b.is_exclusive()
    }

    unsafe fn run_unsafe(&mut self, input: Self::In, world: &crate::prelude::World) -> Self::Out {
        Func::combine(
            input,
            world,
            |input, w| self.a.run_unsafe(input, w),
            |input, w| self.b.run_unsafe(input, w),
        )
    }

    fn run(&mut self, input: Self::In, world: &mut World) -> Self::Out {
        Func::combine_exclusive(
            input,
            world,
            |input, w| self.a.run(input, w),
            |input, w| self.b.run(input, w),
        )
    }

    fn apply_buffers(&mut self, world: &mut crate::prelude::World) {
        self.a.apply_buffers(world);
        self.b.apply_buffers(world);
    }

    fn initialize(&mut self, world: &mut crate::prelude::World) {
        self.a.initialize(world);
        self.b.initialize(world);
        self.component_access.extend(self.a.component_access());
        self.component_access.extend(self.b.component_access());
    }

    fn update_archetype_component_access(&mut self, world: &crate::prelude::World) {
        self.a.update_archetype_component_access(world);
        self.b.update_archetype_component_access(world);

        self.archetype_component_access
            .extend(self.a.archetype_component_access());
        self.archetype_component_access
            .extend(self.b.archetype_component_access());
    }

    fn check_change_tick(&mut self, change_tick: u32) {
        self.a.check_change_tick(change_tick);
        self.b.check_change_tick(change_tick);
    }

    fn get_last_change_tick(&self) -> u32 {
        self.a.get_last_change_tick()
    }

    fn set_last_change_tick(&mut self, last_change_tick: u32) {
        self.a.set_last_change_tick(last_change_tick);
        self.b.set_last_change_tick(last_change_tick);
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
