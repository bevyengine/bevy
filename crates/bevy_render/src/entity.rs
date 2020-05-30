use crate::{mesh::Mesh, Camera, Renderable, OrthographicCamera, PerspectiveCamera, render_resource::resource_name};
use bevy_asset::Handle;
use bevy_derive::EntityArchetype;
use bevy_transform::components::{LocalToWorld, Rotation, Scale, Translation};

#[derive(EntityArchetype, Default)]
pub struct MeshMaterialEntity<T: Default + Send + Sync + 'static> {
    pub mesh: Handle<Mesh>,
    pub material: Handle<T>,
    pub renderable: Renderable,
    pub local_to_world: LocalToWorld,
    pub translation: Translation,
    pub rotation: Rotation,
    pub scale: Scale,
}

#[derive(EntityArchetype)]
pub struct PerspectiveCameraEntity {
    pub camera: Camera,
    pub perspective_camera: PerspectiveCamera,
    pub local_to_world: LocalToWorld,
}

impl Default for PerspectiveCameraEntity {
    fn default() -> Self {
        PerspectiveCameraEntity {
            camera: Camera {
                name: Some(resource_name::uniform::CAMERA.to_string()),
                ..Default::default()
            },
            perspective_camera: Default::default(),
            local_to_world: Default::default(),
        }
    }
    
}

#[derive(EntityArchetype)]
pub struct OrthographicCameraEntity {
    pub camera: Camera,
    pub orthographic_camera: OrthographicCamera,
    pub local_to_world: LocalToWorld,
}

impl OrthographicCameraEntity {
    pub fn ui() -> Self {
        OrthographicCameraEntity {
            camera: Camera {
                // TODO: ui should have its own uniform
                name: Some(resource_name::uniform::CAMERA2D.to_string()),
                ..Default::default()
            },
            orthographic_camera: Default::default(),
            local_to_world: Default::default(),
        }
    }
}

impl Default for OrthographicCameraEntity {
    fn default() -> Self {
        OrthographicCameraEntity {
            camera: Camera {
                name: Some(resource_name::uniform::CAMERA2D.to_string()),
                ..Default::default()
            },
            orthographic_camera: Default::default(),
            local_to_world: Default::default(),
        }
    }
    
}