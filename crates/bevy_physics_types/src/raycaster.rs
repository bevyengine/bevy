use bevy_ecs::component::Component;
use bevy_math::Dir3;

#[derive(Component)]
pub struct RayCaster {
    pub enabled: bool,
    pub origin: Vector,
    pub direction: Dir3,
    pub max_hits: u32,
    pub max_distance: f32,
    pub solid: bool,
    pub ignore_self: bool,
}

pub struct RayHits(pub Vec<RayHitData>);

pub struct RayHitData {
    pub entity: Entity,
    pub distance: f32,
    pub normal: Dir3,
}
