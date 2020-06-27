use crate::Camera;
use legion::{
    entity::Entity,
    prelude::Read,
    systems::{Query, ResMut, SubWorld},
};
use std::collections::HashMap;

#[derive(Default)]
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
        self.cameras.get(name).and_then(|e| e.clone())
    }
}

pub fn active_cameras_system(
    mut active_cameras: ResMut<ActiveCameras>,
    world: &mut SubWorld,
    query: &mut Query<Read<Camera>>,
) {
    for (name, active_camera) in active_cameras.cameras.iter_mut() {
        if let None = active_camera {
            for (camera_entity, camera) in query.iter_entities(world) {
                if let Some(ref current_name) = camera.name {
                    if current_name == name {
                        *active_camera = Some(camera_entity);
                    }
                }
            }
        }
    }
}
