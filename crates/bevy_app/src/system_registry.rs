use crate::App;
use bevy_ecs::{
    prelude::*,
};

impl App {
    /// Runs the supplied system on the [`World`] a single time.
    ///
    /// Calls [`SystemRegistry::run_system`](bevy_ecs::system::SystemRegistry::run_system).
    #[inline]
    pub fn run_system<M, S: IntoSystem<(), (), M> + 'static>(&mut self, system: S) -> &mut Self {
        self.world.run_system(system);
        self
    }
}
