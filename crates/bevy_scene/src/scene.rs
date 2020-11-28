use bevy_ecs::World;
use bevy_reflect::TypeUuid;

#[derive(Debug, TypeUuid)]
#[uuid = "c156503c-edd9-4ec7-8d33-dab392df03cd"]
pub struct Scene {
    pub world: World,
}

impl Scene {
    pub fn new(world: World) -> Self {
        Self { world }
    }
}
