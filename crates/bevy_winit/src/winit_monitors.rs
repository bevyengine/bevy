use std::collections::BTreeMap;
use winit::monitor::MonitorHandle;

use bevy_ecs::entity::Entity;
use bevy_ecs::system::Resource;

/// Stores [`winit`] monitors and their corresponding entities
#[derive(Resource, Debug, Default)]
pub struct WinitMonitors {
    /// Stores [`winit`] monitors and their corresponding entities
    pub monitors: BTreeMap<MonitorHandle, Entity>,
}

impl WinitMonitors {
   pub fn nth(&self, n: usize) -> Option<MonitorHandle> {
        self.monitors.iter().nth(n)
            .map(|(monitor, _)| monitor.clone())
    }

    pub fn find_entity(&self, entity: Entity) -> Option<MonitorHandle> {
        self.monitors.iter().find(|(_, e)| **e == entity)
            .map(|(monitor, _)| monitor.clone())
    }
}