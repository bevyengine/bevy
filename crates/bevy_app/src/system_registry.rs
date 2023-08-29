use crate::App;
use bevy_ecs::prelude::*;
use bevy_ecs::system::{RunSystem, SystemId, SystemRegistryError};

impl App {
    /// Register a system in the [`SystemRegistry`](bevy_ecs::system::SystemRegistry)
    ///
    /// Calls [`SystemRegistry::register_system`](bevy_ecs::system::SystemRegistry::register).
    pub fn register_system<M, S: IntoSystem<(), (), M> + 'static>(
        &mut self,
        system: S,
    ) -> &mut Self {
        self.world.register_system(system);
        self
    }

    /// Removes a registered system in the [`SystemRegistry`](bevy_ecs::system::SystemRegistry).
    ///
    /// Calls [`SystemRegistry::remove_system`](bevy_ecs::system::SystemRegistry::remove).
    #[inline]
    pub fn remove_system(&mut self, id: SystemId) {
        self.world.remove_system(id);
    }

    /// Runs the supplied system on the [`World`] a single time.
    ///
    /// Calls [`RunSystem::run_system`](bevy_ecs::system::RunSystem::run_system).
    #[inline]
    pub fn run_system<M, S: IntoSystem<(), (), M> + 'static>(&mut self, system: S) -> &mut Self {
        self.world.run_system(system);
        self
    }

    /// Run the systems corresponding to the id.
    ///
    /// Calls [`SystemRegistry::run_system_by_id`](bevy_ecs::system::SystemRegistry::run_by_id).
    #[inline]
    pub fn run_system_by_id(&mut self, system_id: SystemId) -> Result<(), SystemRegistryError> {
        self.world.run_system_by_id(system_id)
    }
}
