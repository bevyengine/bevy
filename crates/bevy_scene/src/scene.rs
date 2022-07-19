use bevy_ecs::world::World;
use bevy_reflect::TypeUuid;

/// To spawn a scene, you can use either:
/// * [`SceneSpawner::spawn`](crate::SceneSpawner::spawn)
/// * adding the [`SceneBundle`](crate::SceneBundle) to an entity
/// * adding the [`Handle<Scene>`](bevy_asset::Handle) to an entity (the scene will only be
/// visible if the entity already has [`Transform`](bevy_transform::components::Transform) and
/// [`GlobalTransform`](bevy_transform::components::GlobalTransform) components)
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
