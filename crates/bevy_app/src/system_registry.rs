use crate::App;
use bevy_ecs::prelude::*;
use bevy_ecs::system::{Callback, SystemRegistryError};

impl App {
    /// Registers the supplied system in the [`SystemRegistry`](bevy_ecs::system::SystemRegistry) resource.
    ///
    /// Calls the method of the same name on [`SystemRegistry`](bevy_ecs::system::SystemRegistry).
    #[inline]
    pub fn register_system<Params, S: IntoSystem<(), (), Params> + 'static>(
        &mut self,
        system: S,
    ) -> &mut Self {
        self.world.register_system(system);
        self
    }

    /// Register system a system with any number of [`SystemLabel`]s.
    ///
    /// Calls the method of the same name on [`SystemRegistry`].
    ///
    /// [`SystemRegistry`]: bevy_ecs::system::SystemRegistry
    pub fn register_system_with_labels<
        Params,
        S: IntoSystem<(), (), Params> + 'static,
        LI: IntoIterator<Item = L>,
        L: SystemLabel,
    >(
        &mut self,
        system: S,
        labels: LI,
    ) -> &mut Self {
        self.world.register_system_with_labels(system, labels);
        self
    }

    /// Runs the supplied system on the [`World`] a single time.
    ///
    /// Calls the method of the same name on [`SystemRegistry`](bevy_ecs::system::SystemRegistry).
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
    /// Calls the method of the same name on [`SystemRegistry`](bevy_ecs::system::SystemRegistry).
    #[inline]
    pub fn run_systems_by_label<L: SystemLabel>(
        &mut self,
        label: L,
    ) -> Result<(), SystemRegistryError> {
        self.world.run_systems_by_label(label)
    }

    /// Run the systems corresponding to the label stored in the provided [`Callback`]
    ///
    /// Calls the method of the same name on [`SystemRegistry`](bevy_ecs::system::SystemRegistry).
    #[inline]
    pub fn run_callback(&mut self, callback: Callback) -> Result<(), SystemRegistryError> {
        self.world.run_callback(callback)
    }
}
