use crate::{
    component::Tick,
    system::{
        FunctionSystem, IntoResult, IntoSystem, IsFunctionSystem, RunSystemError, System, SystemIn, SystemInput, SystemParamFunction
    },
    world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld, World},
};

use super::OptionPacket;

pub trait IntoPacketSystem<In: SystemInput, Out, Marker>: Sized {
    type System: System<In = In, Out = ()>;
    fn into_system(this: Self) -> Self::System;
}
pub struct FunctionPacketSystem<Marker, F>
where
    F: SystemParamFunction<Marker>,
{
    inner: FunctionSystem<Marker, F::Out, F>,
}
impl<Marker, F> IntoPacketSystem<F::In, F::Out, (IsFunctionSystem, Marker)> for F
where
    Marker: 'static,
    F: SystemParamFunction<Marker>,
    F::Out: OptionPacket,
{
    type System = FunctionPacketSystem<Marker, F>;
    fn into_system(func: Self) -> Self::System {
        let inner = IntoSystem::into_system(func);
        return FunctionPacketSystem { inner };
    }
}
impl<Marker, F> System for FunctionPacketSystem<Marker, F>
where
    Marker: 'static,
    F: SystemParamFunction<Marker>,
    F::Out: OptionPacket,
{
    type In = F::In;
    type Out = ();

    #[inline]
    fn is_send(&self) -> bool {
        self.inner.is_send()
    }

    #[inline]
    fn is_exclusive(&self) -> bool {
        self.inner.is_exclusive()
    }

    #[inline]
    fn has_deferred(&self) -> bool {
        self.inner.has_deferred()
    }
    fn run(&mut self, input: SystemIn<'_, Self>, world: &mut World) -> Result<Self::Out, RunSystemError> {
        let out = self.inner.run(input, world)?;
        let rv = out.run(world);
        return IntoResult::into_result(rv);
    }

    #[inline]
    fn apply_deferred(&mut self, world: &mut World) {
        self.inner.apply_deferred(world);
    }

    #[inline]
    fn queue_deferred(&mut self, world: DeferredWorld) {
        self.inner.queue_deferred(world);
    }

    fn get_last_run(&self) -> Tick {
        self.inner.get_last_run()
    }

    fn set_last_run(&mut self, last_run: Tick) {
        self.inner.set_last_run(last_run);
    }

    fn flags(&self) -> crate::system::SystemStateFlags {
        self.inner.flags()
    }

    fn name(&self) -> bevy_utils::prelude::DebugName {
        self.inner.name()
    }

    unsafe fn validate_param_unsafe(
        &mut self,
        world: UnsafeWorldCell,
    ) -> Result<(), crate::system::SystemParamValidationError> {
        self.inner.validate_param_unsafe(world)
    }

    fn initialize(&mut self, world: &mut World) -> crate::query::FilteredAccessSet<crate::component::ComponentId> {
        self.inner.initialize(world)
    }

    fn check_change_tick(&mut self, check: crate::component::CheckChangeTicks) {
        self.inner.check_change_tick(check)
    }

    unsafe fn run_unsafe(
        &mut self,
        _input: SystemIn<'_, Self>,
        _world: UnsafeWorldCell,
    ) -> Result<Self::Out, RunSystemError> {
        unimplemented!("no multithreading, use run")
    }
}
