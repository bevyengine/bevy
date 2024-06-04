use winit::monitor::MonitorHandle;

use bevy_ecs::entity::Entity;
use bevy_ecs::system::Resource;

/// Stores [`winit`] monitors and their corresponding entities
#[derive(Resource, Debug, Default)]
pub struct WinitMonitors {
    /// Stores [`winit`] monitors and their corresponding entities
    // we can't use a `HashMap` here because `MonitorHandle` doesn't implement `Hash` :(
    pub monitors: Vec<(MonitorHandle, Entity)>,
}
