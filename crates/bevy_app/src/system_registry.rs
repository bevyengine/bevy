use crate::App;
use bevy_ecs::prelude::*;
use bevy_ecs::system::{Callback, SystemRegistryError};

impl App {
    /// Register a system with any number of [`SystemLabel`]s.
    ///
    /// Calls [`SystemRegistry::register_system`](bevy_ecs::system::SystemRegistry::register_system).
    pub fn register_system<
        Params,
        S: IntoSystem<(), (), Params> + 'static,
        LI: IntoIterator<Item = L>,
        L: SystemLabel,
    >(
        &mut self,
        system: S,
        labels: LI,
    ) -> &mut Self {
        self.world.register_system(system, labels);
        self
    }

    /// Runs the supplied system on the [`World`] a single time.
    ///
    /// Calls [`SystemRegistry::run_system`](bevy_ecs::system::SystemRegistry::run_system).
    #[inline]
    pub fn run_system<Params, S: IntoSystem<(), (), Params> + 'static>(
        &mut self,
        system: S,
    ) -> &mut Self {
        self.world.run_system(system);
        self
    }

    /// Runs the systems corresponding to the supplied [`SystemLabel`] on the [`World`] a single time.
    ///
    /// Calls [`SystemRegistry::run_systems_by_label`](bevy_ecs::system::SystemRegistry::run_systems_by_label).
    #[inline]
    pub fn run_systems_by_label<L: SystemLabel>(
        &mut self,
        label: L,
    ) -> Result<(), SystemRegistryError> {
        self.world.run_systems_by_label(label)
    }

    /// Run the systems corresponding to the label stored in the provided [`Callback`]
    ///
    /// Calls [`SystemRegistry::run_callback`](bevy_ecs::system::SystemRegistry::run_callback).
    #[inline]
    pub fn run_callback(&mut self, callback: Callback) -> Result<(), SystemRegistryError> {
        self.world.run_callback(callback)
    }
}
