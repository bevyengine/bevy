use bevy_utils::synccell::SyncCell;

use super::{ReadOnlySystem, System};

pub trait Adapt<S: System>: Send {
    /// The [input](System::In) type for an [`AdapterSystem`].
    type In;
    /// The [output](System::Out) type for an [`AdapterSystem`].
    type Out;

    /// When used in an [`AdapterSystem`], this function customizes how the system
    /// is run and how its inputs/outputs are adapted.
    fn adapt(&mut self, input: Self::In, run_system: impl FnOnce(S::In) -> S::Out) -> Self::Out;
}

#[derive(Clone)]
pub struct AdapterSystem<Func, S> {
    func: SyncCell<Func>,
    system: S,
}

impl<Func, S> AdapterSystem<Func, S> {
    pub fn new(func: Func, system: S) -> Self {
        Self {
            func: SyncCell::new(func),
            system,
        }
    }
}

impl<Func, S> System for AdapterSystem<Func, S>
where
    Func: Adapt<S> + 'static,
    S: System,
{
    type In = Func::In;
    type Out = Func::Out;

    fn name(&self) -> std::borrow::Cow<'static, str> {
        self.system.name()
    }

    fn type_id(&self) -> std::any::TypeId {
        self.system.type_id()
    }

    fn component_access(&self) -> &crate::query::Access<crate::component::ComponentId> {
        self.system.component_access()
    }

    #[inline]
    fn archetype_component_access(
        &self,
    ) -> &crate::query::Access<crate::archetype::ArchetypeComponentId> {
        self.system.archetype_component_access()
    }

    fn is_send(&self) -> bool {
        self.system.is_send()
    }

    fn is_exclusive(&self) -> bool {
        self.system.is_exclusive()
    }

    #[inline]
    unsafe fn run_unsafe(&mut self, input: Self::In, world: &crate::prelude::World) -> Self::Out {
        // SAFETY: `system.run_unsafe` has the same invariants as `self.run_unsafe`.
        self.func
            .get()
            .adapt(input, |input| self.system.run_unsafe(input, world))
    }

    #[inline]
    fn run(&mut self, input: Self::In, world: &mut crate::prelude::World) -> Self::Out {
        self.func
            .get()
            .adapt(input, |input| self.system.run(input, world))
    }

    #[inline]
    fn apply_buffers(&mut self, world: &mut crate::prelude::World) {
        self.system.apply_buffers(world);
    }

    fn initialize(&mut self, world: &mut crate::prelude::World) {
        self.system.initialize(world);
    }

    #[inline]
    fn update_archetype_component_access(&mut self, world: &crate::prelude::World) {
        self.system.update_archetype_component_access(world);
    }

    fn check_change_tick(&mut self, change_tick: crate::component::Tick) {
        self.system.check_change_tick(change_tick);
    }

    fn get_last_run(&self) -> crate::component::Tick {
        self.system.get_last_run()
    }

    fn set_last_run(&mut self, last_run: crate::component::Tick) {
        self.system.set_last_run(last_run);
    }

    fn default_system_sets(&self) -> Vec<Box<dyn crate::schedule::SystemSet>> {
        self.system.default_system_sets()
    }
}

// SAFETY: The inner system is read-only.
unsafe impl<Func, S> ReadOnlySystem for AdapterSystem<Func, S>
where
    Func: Adapt<S> + 'static,
    S: ReadOnlySystem,
{
}

impl<F, S, Out> Adapt<S> for F
where
    S: System,
    F: Send + FnMut(S::Out) -> Out,
{
    type In = S::In;
    type Out = Out;

    fn adapt(&mut self, input: S::In, run_system: impl FnOnce(S::In) -> S::Out) -> Out {
        self(run_system(input))
    }
}
