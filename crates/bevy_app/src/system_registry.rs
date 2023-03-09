use crate::App;
use bevy_ecs::prelude::*;
use bevy_ecs::system::{Callback, SystemRegistryError};

impl App {
    /// Register a system with its [`SystemTypeSet`].
    ///
    /// Calls [`SystemRegistry::register_system_with_type_set`](bevy_ecs::system::SystemRegistry::register_system_with_type_set).
    pub fn register_system_with_type_set<Params, S: IntoSystem<(), (), Params> + 'static>(
        &mut self,
        system: S,
    ) -> &mut Self {
        self.world.register_system_with_type_set(system);
        self
    }

    /// Register a system with any number of [`SystemLabel`]s.
    ///
    /// Calls [`SystemRegistry::register_system`](bevy_ecs::system::SystemRegistry::register_system).
    pub fn register_system<
        Params,
        S: IntoSystem<(), (), Params> + 'static,
        SSI: IntoIterator<Item = SS>,
        SS: SystemSet,
    >(
        &mut self,
        system: S,
        sets: SSI,
    ) -> &mut Self {
        self.world.register_system(system, sets);
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
    pub fn run_systems_by_set<S: SystemSet>(&mut self, set: S) -> Result<(), SystemRegistryError> {
        self.world.run_systems_by_set(set)
    }

    /// Run the systems corresponding to the label stored in the provided [`Callback`]
    ///
    /// Calls [`SystemRegistry::run_callback`](bevy_ecs::system::SystemRegistry::run_callback).
    #[inline]
    pub fn run_callback(&mut self, callback: Callback) -> Result<(), SystemRegistryError> {
        self.world.run_callback(callback)
    }
}
