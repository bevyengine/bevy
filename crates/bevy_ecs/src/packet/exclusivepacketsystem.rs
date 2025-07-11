use crate::{component::{ComponentId, Tick}, system::{ExclusiveFunctionSystem, ExclusiveSystemParamFunction, IntoSystem, IsExclusiveFunctionSystem, System, SystemIn}, world::{unsafe_world_cell::UnsafeWorldCell, World}};

use super::{packetsystem::IntoPacketSystem, OptionPacket};

pub struct ExclusivePacketSystem<Marker, F>
where
    F: ExclusiveSystemParamFunction<Marker>,
{
    inner: ExclusiveFunctionSystem<Marker, F::Out, F>,
}
impl<Marker, F> IntoPacketSystem<F::In, F::Out, (IsExclusiveFunctionSystem, Marker)> for F
where
    Marker: 'static,
    F: ExclusiveSystemParamFunction<Marker>,
    F::Out: OptionPacket,
{
    type System = ExclusivePacketSystem<Marker, F>;
    fn into_system(func: Self) -> Self::System {
        ExclusivePacketSystem {
            inner: IntoSystem::into_system(func)
        }
    }
}

impl<Marker, F> System for ExclusivePacketSystem<Marker, F>
where
    Marker: 'static,
    F: ExclusiveSystemParamFunction<Marker>,
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

    fn run(&mut self, input: SystemIn<'_, Self>, world: &mut World) -> Result<Self::Out, crate::system::RunSystemError> {
        let out = <ExclusiveFunctionSystem<Marker, F::Out, F> as System>::run(&mut self.inner, input, world)?;
        out.run(world);
        Ok(())
    }

    #[inline]
    fn apply_deferred(&mut self, world: &mut World) {
        self.inner.apply_deferred(world);
    }

    #[inline]
    fn queue_deferred(&mut self, world: crate::world::DeferredWorld) {
        self.inner.queue_deferred(world);
    }

    fn get_last_run(&self) -> Tick {
        self.inner.get_last_run()
    }

    fn set_last_run(&mut self, last_run: Tick) {
        self.inner.set_last_run(last_run);
    }

    fn name(&self) -> bevy_utils::prelude::DebugName {
        self.inner.name()
    }

    fn flags(&self) -> crate::system::SystemStateFlags {
        self.inner.flags()
    }

    unsafe fn run_unsafe(
        &mut self,
        _input: SystemIn<'_, Self>,
        _world: UnsafeWorldCell,
    ) -> Result<Self::Out, crate::system::RunSystemError> {
        panic!("exclusive system")
    }

    unsafe fn validate_param_unsafe(
        &mut self,
        world: UnsafeWorldCell,
    ) -> Result<(), crate::system::SystemParamValidationError> {
        self.inner.validate_param_unsafe(world)
    }

    fn initialize(&mut self, world: &mut World) -> crate::query::FilteredAccessSet<ComponentId> {
        self.inner.initialize(world)
    }

    fn check_change_tick(&mut self, check: crate::component::CheckChangeTicks) {
        self.inner.check_change_tick(check)
    }

}
