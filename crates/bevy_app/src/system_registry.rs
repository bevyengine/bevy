use crate::App;
use bevy_ecs::{
    prelude::*,
    system::{SystemId, SystemRegistryError},
};

impl App {
    /// Register a system.
    ///
    /// Calls [`SystemRegistry::register_system`](bevy_ecs::system::SystemRegistry::register_system).
    pub fn register_system<M, S: IntoSystem<(), (), M> + 'static>(
        &mut self,
        system: S,
    ) -> &mut Self {
        self.world.register_system(system);
        self
    }

    /// Runs the supplied system on the [`World`] a single time.
    ///
    /// Calls [`SystemRegistry::run_system`](bevy_ecs::system::SystemRegistry::run_system).
    #[inline]
    pub fn run_system<M, S: IntoSystem<(), (), M> + 'static>(&mut self, system: S) -> &mut Self {
        self.world.run_system(system);
        self
    }

    /// Run the systems corresponding to the label stored in the provided [`Callback`]
    ///
    /// Calls [`SystemRegistry::run_callback`](bevy_ecs::system::SystemRegistry::run_callback).
    #[inline]
    pub fn run_system_by_id(&mut self, system_id: SystemId) -> Result<(), SystemRegistryError> {
        self.world.run_system_by_id(system_id)
    }
}
