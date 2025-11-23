use bevy_ecs::prelude::Component;

/// Identifies an entity as a force region.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ForceRegion;
