use crate::{
    camera::{Camera, OrthographicProjection, PerspectiveProjection, VisibleEntities},
    pipeline::RenderPipelines,
    render_graph::base,
    Draw, Mesh,
};
use base::MainPass;
use bevy_asset::Handle;
use bevy_ecs::Bundle;
use bevy_transform::components::{Rotation, Scale, Transform, Translation};

/// A component bundle for "mesh" entities
#[derive(Bundle, Default)]
pub struct MeshComponents {
    pub mesh: Handle<Mesh>,
    pub draw: Draw,
    pub render_pipelines: RenderPipelines,
    pub main_pass: MainPass,
    pub transform: Transform,
    pub translation: Translation,
    pub rotation: Rotation,
    pub scale: Scale,
}

/// A component bundle for "3d camera" entities
#[derive(Bundle)]
pub struct Camera3dComponents {
    pub camera: Camera,
    pub perspective_projection: PerspectiveProjection,
    pub visible_entities: VisibleEntities,
    pub transform: Transform,
    pub translation: Translation,
    pub rotation: Rotation,
    pub scale: Scale,
}

impl Default for Camera3dComponents {
    fn default() -> Self {
        Camera3dComponents {
            camera: Camera {
                name: Some(base::camera::CAMERA3D.to_string()),
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

/// A component bundle for "2d camera" entities
#[derive(Bundle)]
pub struct Camera2dComponents {
    pub camera: Camera,
    pub orthographic_projection: OrthographicProjection,
    pub visible_entities: VisibleEntities,
    pub transform: Transform,
    pub translation: Translation,
    pub rotation: Rotation,
    pub scale: Scale,
}

impl Default for Camera2dComponents {
    fn default() -> Self {
        // we want 0 to be "closest" and +far to be "farthest" in 2d, so we offset
        // the camera's translation by far and use a right handed coordinate system
        let far = 1000.0;
        Camera2dComponents {
            camera: Camera {
                name: Some(base::camera::CAMERA2D.to_string()),
                ..Default::default()
            },
            orthographic_projection: OrthographicProjection {
                far,
                ..Default::default()
            },
            visible_entities: Default::default(),
            transform: Default::default(),
            translation: Translation::new(0.0, 0.0, far - 0.1),
            rotation: Default::default(),
            scale: Default::default(),
        }
    }
}
