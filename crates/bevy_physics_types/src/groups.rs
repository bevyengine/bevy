use bevy_ecs::prelude::Component;

/// Defines a collision group for coarse filtering.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionGroups {
    pub memberships: u32,
    pub filters: u32,
}

impl Default for CollisionGroups {
    fn default() -> Self {
        Self {
            memberships: 0xFFFF_FFFF,
            filters: 0xFFFF_FFFF,
        }
    }
}
