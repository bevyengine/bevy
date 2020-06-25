use crate::{
    base_render_graph, draw::Draw, mesh::Mesh, pipeline::RenderPipelines, Camera,
    OrthographicProjection, PerspectiveProjection, WindowOrigin, VisibleEntities,
};
use bevy_asset::Handle;
use bevy_derive::EntityArchetype;
use bevy_transform::components::{Rotation, Scale, Transform, Translation};

#[derive(EntityArchetype, Default)]
pub struct MeshMaterialEntity<T: Default + Send + Sync + 'static> {
    pub mesh: Handle<Mesh>,
    pub material: Handle<T>,
    pub draw: Draw,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub translation: Translation,
    pub rotation: Rotation,
    pub scale: Scale,
}

#[derive(EntityArchetype)]
pub struct PerspectiveCameraEntity {
    pub camera: Camera,
    pub perspective_projection: PerspectiveProjection,
    pub visible_entities: VisibleEntities,
    pub transform: Transform,
    pub translation: Translation,
    pub rotation: Rotation,
    pub scale: Scale,
}

impl Default for PerspectiveCameraEntity {
    fn default() -> Self {
        PerspectiveCameraEntity {
            camera: Camera {
                name: Some(base_render_graph::camera::CAMERA.to_string()),
                ..Default::default()
            },
            perspective_projection: Default::default(),
            visible_entities: Default::default(),
            transform: Default::default(),
            translation: Default::default(),
            rotation: Default::default(),
            scale: Default::default(),
        }
    }
}

#[derive(EntityArchetype)]
pub struct OrthographicCameraEntity {
    pub camera: Camera,
    pub orthographic_projection: OrthographicProjection,
    pub visible_entities: VisibleEntities,
    pub transform: Transform,
    pub translation: Translation,
    pub rotation: Rotation,
    pub scale: Scale,
}

impl OrthographicCameraEntity {
    pub fn ui() -> Self {
        OrthographicCameraEntity {
            camera: Camera {
                name: Some("UiCamera".to_string()),
                ..Default::default()
            },
            orthographic_projection: OrthographicProjection {
                window_origin: WindowOrigin::Center,
                ..Default::default()
            },
            visible_entities: Default::default(),
            transform: Default::default(),
            translation: Default::default(),
            rotation: Default::default(),
            scale: Default::default(),
        }
    }
}

impl Default for OrthographicCameraEntity {
    fn default() -> Self {
        OrthographicCameraEntity {
            camera: Camera {
                name: Some(base_render_graph::camera::CAMERA2D.to_string()),
                ..Default::default()
            },
            orthographic_projection: Default::default(),
            visible_entities: Default::default(),
            transform: Default::default(),
            translation: Default::default(),
            rotation: Default::default(),
            scale: Default::default(),
        }
    }
}
