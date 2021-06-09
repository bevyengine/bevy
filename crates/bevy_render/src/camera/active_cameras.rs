use crate::renderer::RenderResourceBindings;

use super::Camera;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    system::{Query, ResMut},
};
use bevy_utils::HashMap;

#[derive(Component, Debug, Default)]
pub struct ActiveCamera {
    pub name: String,
    pub entity: Option<Entity>,
    pub bindings: RenderResourceBindings,
}

#[derive(Debug, Default)]
pub struct ActiveCameras {
    cameras: HashMap<String, ActiveCamera>,
}

impl ActiveCameras {
    pub fn add(&mut self, name: &str) {
        self.cameras.insert(
            name.to_string(),
            ActiveCamera {
                name: name.to_string(),
                ..Default::default()
            },
        );
    }

    pub fn get(&self, name: &str) -> Option<&ActiveCamera> {
        self.cameras.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut ActiveCamera> {
        self.cameras.get_mut(name)
    }

    pub fn remove(&mut self, name: &str) -> Option<ActiveCamera> {
        self.cameras.remove(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ActiveCamera> {
        self.cameras.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ActiveCamera> {
        self.cameras.values_mut()
    }
}

pub fn active_cameras_system(
    mut active_cameras: ResMut<ActiveCameras>,
    query: Query<(Entity, &Camera)>,
) {
    for (name, active_camera) in active_cameras.cameras.iter_mut() {
        if active_camera
            .entity
            .map_or(false, |entity| query.get(entity).is_err())
        {
            active_camera.entity = None;
        }

        if active_camera.entity.is_none() {
            for (camera_entity, camera) in query.iter() {
                if let Some(ref current_name) = camera.name {
                    if current_name == name {
                        active_camera.entity = Some(camera_entity);
                    }
                }
            }
        }
    }
}
