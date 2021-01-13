use std::any::TypeId;

use super::{Diagnostic, DiagnosticId, Diagnostics};
use bevy_app::prelude::*;
use bevy_ecs::{
    ArchetypeComponent, BoxedSystem, Resources, System, SystemId, ThreadLocalExecution, TypeAccess,
    World,
};
use bevy_utils::Instant;

pub trait AppBuilderMeasuredSystemExt {
    /// Add a system and record its execution time in [Diagnostics].
    ///
    /// # Example
    /// ```
    /// use bevy::{diagnostic::{AppBuilderMeasuredSystemExt, LogDiagnosticsPlugin}, prelude::*};
    ///
    /// pub fn timed_system() {
    ///     std::thread::sleep(std::time::Duration::new(0, 5000000));
    /// }
    ///
    /// fn main() {
    ///     App::build()
    ///         .add_plugins(DefaultPlugins)
    ///         .add_plugin(LogDiagnosticsPlugin::default())
    ///         .add_measured_system("timed_system", timed_system.system())
    ///         .add_system(timed_system.system())
    ///         .run();
    /// }
    /// ```
    fn add_measured_system<S: System<In = (), Out = ()>>(
        &mut self,
        name: &str,
        system: S,
    ) -> &mut Self;
}

impl AppBuilderMeasuredSystemExt for AppBuilder {
    fn add_measured_system<S: System<In = (), Out = ()>>(
        &mut self,
        name: &str,
        system: S,
    ) -> &mut Self {
        let resources = self.resources();
        let mut diagnostics = resources.get_mut::<Diagnostics>().unwrap();
        let measured_system = MeasuredSystem::new(name, Box::new(system), &mut *diagnostics);
        drop(diagnostics);
        self.add_system(measured_system)
    }
}

impl System for MeasuredSystem {
    type In = ();
    type Out = ();

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
        let mut diagnostics = resources.get_mut::<Diagnostics>().unwrap();
        diagnostics.add_measurement(self.diagnostic_id, now.elapsed().as_secs_f64());
        output
    }

    fn run_thread_local(&mut self, world: &mut World, resources: &mut Resources) {
        self.system.run_thread_local(world, resources)
    }

    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        self.system.initialize(world, resources)
    }
}

struct MeasuredSystem {
    system: BoxedSystem,
    diagnostic_id: DiagnosticId,
    resource_access: TypeAccess<TypeId>,
}

impl MeasuredSystem {
    fn new(name: &str, system: BoxedSystem, diagnostics: &mut Diagnostics) -> Self {
        let diagnostic_id = DiagnosticId::default();
        diagnostics.add(Diagnostic::new(diagnostic_id, name, 20));
        let mut resource_access = system.resource_access().clone();
        resource_access.add_write(TypeId::of::<Diagnostics>());
        Self {
            system,
            diagnostic_id,
            resource_access,
        }
    }
}
