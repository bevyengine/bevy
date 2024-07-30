use winit::monitor::MonitorHandle;

use bevy_ecs::entity::Entity;
use bevy_ecs::system::Resource;

/// Stores [`winit`] monitors and their corresponding entities
#[derive(Resource, Debug, Default)]
pub struct WinitMonitors {
    /// Stores [`winit`] monitors and their corresponding entities
    // We can't use a `BtreeMap` here because clippy complains about using `MonitorHandle` as a key
    // on some platforms. Using a `Vec` is fine because we don't expect to have a large number of
    // monitors and avoids having to audit the code for `MonitorHandle` equality.
    pub(crate) monitors: Vec<(MonitorHandle, Entity)>,
}

impl WinitMonitors {
    pub fn nth(&self, n: usize) -> Option<MonitorHandle> {
        self.monitors.get(n).map(|(monitor, _)| monitor.clone())
    }

    pub fn find_entity(&self, entity: Entity) -> Option<MonitorHandle> {
        self.monitors
            .iter()
            .find(|(_, e)| *e == entity)
            .map(|(monitor, _)| monitor.clone())
    }
}
