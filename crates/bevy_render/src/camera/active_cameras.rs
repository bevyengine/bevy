use super::Camera;
use bevy_ecs::{Entity, Query, ResMut};
use bevy_utils::HashMap;

#[derive(Debug, Default)]
pub struct ActiveCameras {
    pub cameras: HashMap<String, Option<Entity>>,
}

impl ActiveCameras {
    pub fn add(&mut self, name: &str) {
        self.cameras.insert(name.to_string(), None);
    }

    pub fn set(&mut self, name: &str, entity: Entity) {
        self.cameras.insert(name.to_string(), Some(entity));
    }

    pub fn get(&self, name: &str) -> Option<Entity> {
        self.cameras.get(name).and_then(|e| *e)
    }
}

pub fn active_cameras_system(
    mut active_cameras: ResMut<ActiveCameras>,
    query: Query<(Entity, &Camera)>,
) {
    for (name, active_camera) in active_cameras.cameras.iter_mut() {
        if active_camera.is_none() {
            for (camera_entity, camera) in query.iter() {
                if let Some(ref current_name) = camera.name {
                    if current_name == name {
                        *active_camera = Some(camera_entity);
                    }
                }
            }
        }
    }
}
