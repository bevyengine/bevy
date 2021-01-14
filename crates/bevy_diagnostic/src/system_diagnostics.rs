use std::any::TypeId;

use super::{Diagnostic, DiagnosticId, Diagnostics};
use bevy_ecs::{
    ArchetypeComponent, Resources, System, SystemId, ThreadLocalExecution, TypeAccess, World,
};
use bevy_utils::Instant;

pub trait MeasuredSystemExt<In, Out>: System<In = In, Out = Out> + Sized {
    /// Add a system and record its execution time in [Diagnostics].
    ///
    /// # Example
    /// ```
    /// use bevy::{
    ///     diagnostic::{LogDiagnosticsPlugin, MeasuredSystemExt},
    ///     prelude::*,
    /// };
    ///
    /// pub fn timed_system() {
    ///     std::thread::sleep(std::time::Duration::new(0, 5000000));
    /// }
    ///
    /// fn main() {
    ///     App::build()
    ///         .add_plugins(DefaultPlugins)
    ///         .add_plugin(LogDiagnosticsPlugin::default())
    ///         .add_system(timed_system.system().measured())
    ///         .run();
    /// }
    /// ```
    fn measured(self) -> MeasuredSystem<Self>;
}

impl<In: 'static, Out: 'static, Sys: System<In = In, Out = Out> + Sized> MeasuredSystemExt<In, Out>
    for Sys
{
    fn measured(self) -> MeasuredSystem<Sys> {
        MeasuredSystem {
            diagnostic_id: Default::default(),
            resource_access: Default::default(),
            system: self,
        }
    }
}

pub struct MeasuredSystem<Sys> {
    system: Sys,
    diagnostic_id: DiagnosticId,
    resource_access: TypeAccess<TypeId>,
}

impl<In: 'static, Out: 'static, Sys : System<In = In, Out = Out>> System for MeasuredSystem<Sys> {
    type In = In;
    type Out = Out;

    fn name(&self) -> std::borrow::Cow<'static, str> {
        self.system.name()
    }

    fn id(&self) -> SystemId {
        self.system.id()
    }

    fn update(&mut self, world: &World) {
        self.system.update(world)
    }

    fn archetype_component_access(&self) -> &TypeAccess<ArchetypeComponent> {
        self.system.archetype_component_access()
    }

    fn resource_access(&self) -> &TypeAccess<TypeId> {
        &self.resource_access
    }

    fn thread_local_execution(&self) -> ThreadLocalExecution {
        self.system.thread_local_execution()
    }

    unsafe fn run_unsafe(
        &mut self,
        input: Self::In,
        world: &World,
        resources: &Resources,
    ) -> Option<Self::Out> {
        let now = Instant::now();
        let output = self.system.run_unsafe(input, world, resources);
        let elapsed = now.elapsed().as_secs_f64();
        let mut diagnostics = resources.get_mut::<Diagnostics>().unwrap();
        diagnostics.add_measurement(self.diagnostic_id, elapsed);
        output
    }

    fn run_thread_local(&mut self, world: &mut World, resources: &mut Resources) {
        self.system.run_thread_local(world, resources)
    }

    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        self.system.initialize(world, resources);
        self.resource_access = self.system.resource_access().clone();
        if self
            .resource_access
            .is_read_or_write(&TypeId::of::<Diagnostics>())
        {
            panic!(
                "System `{}` has a `Res<Diagnostics>` or `ResMut<Diagnostics>` parameter, \
                it cannot be made into a measured system.",
                self.name()
            );
        }
        self.resource_access.add_write(TypeId::of::<Diagnostics>());

        let mut diagnostics = resources.get_mut::<Diagnostics>().unwrap();
        diagnostics.add(Diagnostic::new(self.diagnostic_id, &self.name(), 20));
    }
}
