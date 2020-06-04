use crate::{
    base_render_graph, mesh::Mesh, Camera, OrthographicProjection, PerspectiveProjection,
    Renderable, WindowOrigin,
};
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
    pub perspective_projection: PerspectiveProjection,
    pub local_to_world: LocalToWorld,
    pub translation: Translation,
    pub rotation: Rotation,
}

impl Default for PerspectiveCameraEntity {
    fn default() -> Self {
        PerspectiveCameraEntity {
            camera: Camera {
                name: Some(base_render_graph::uniform::CAMERA.to_string()),
                ..Default::default()
            },
            perspective_projection: Default::default(),
            local_to_world: Default::default(),
            translation: Default::default(),
            rotation: Default::default(),
        }
    }
}

#[derive(EntityArchetype)]
pub struct OrthographicCameraEntity {
    pub camera: Camera,
    pub orthographic_projection: OrthographicProjection,
    pub local_to_world: LocalToWorld,
    pub translation: Translation,
    pub rotation: Rotation,
}

impl OrthographicCameraEntity {
    pub fn ui() -> Self {
        OrthographicCameraEntity {
            camera: Camera {
                name: Some("UiCamera".to_string()),
                ..Default::default()
            },
            orthographic_projection: OrthographicProjection {
                window_origin: WindowOrigin::BottomLeft,
                ..Default::default()
            },
            local_to_world: Default::default(),
            translation: Default::default(),
            rotation: Default::default(),
        }
    }
}

impl Default for OrthographicCameraEntity {
    fn default() -> Self {
        OrthographicCameraEntity {
            camera: Camera {
                name: Some(base_render_graph::uniform::CAMERA2D.to_string()),
                ..Default::default()
            },
            orthographic_projection: Default::default(),
            local_to_world: Default::default(),
            translation: Default::default(),
            rotation: Default::default(),
        }
    }
}
