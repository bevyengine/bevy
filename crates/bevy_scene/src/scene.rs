use bevy_ecs::world::Turtle;
use bevy_reflect::TypeUuid;

#[derive(Debug, TypeUuid)]
#[uuid = "c156503c-edd9-4ec7-8d33-dab392df03cd"]
pub struct Scene {
    pub turtle: Turtle,
}
