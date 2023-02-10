use std::{borrow::Cow, marker::PhantomData};

use crate::{
    archetype::ArchetypeComponentId, component::ComponentId, prelude::World, query::Access,
};

use super::{ReadOnlySystem, System};

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
