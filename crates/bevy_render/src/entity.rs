use crate::{
    camera::{
        Camera, OrthographicProjection, PerspectiveProjection, ScaledOrthographicProjection,
        VisibleEntities,
    },
    pipeline::RenderPipelines,
    prelude::Visible,
    render_graph::base,
    Draw, Mesh,
};
use base::MainPass;
use bevy_asset::Handle;
use bevy_ecs::Bundle;
use bevy_transform::components::{GlobalTransform, Transform};

/// A component bundle for "mesh" entities
#[derive(Bundle, Default)]
pub struct MeshBundle {
    pub mesh: Handle<Mesh>,
    pub draw: Draw,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    pub main_pass: MainPass,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

/// A component bundle for "3d camera" entities
///
/// Uses a perspective projection.
#[derive(Bundle)]
pub struct Camera3dBundle {
    pub camera: Camera,
    pub perspective_projection: PerspectiveProjection,
    pub visible_entities: VisibleEntities,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for Camera3dBundle {
    fn default() -> Self {
        Camera3dBundle {
            camera: Camera {
                name: Some(base::camera::CAMERA_3D.to_string()),
                ..Default::default()
            },
            perspective_projection: Default::default(),
            visible_entities: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

/// A component bundle for "2d camera" entities
///
/// Uses an orthographic projection mapped to window pixels.
///
/// Use this for pixel-art / sprite-based 2D games.
#[derive(Bundle)]
pub struct Camera2dBundle {
    pub camera: Camera,
    pub orthographic_projection: OrthographicProjection,
    pub visible_entities: VisibleEntities,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for Camera2dBundle {
    fn default() -> Self {
        // we want 0 to be "closest" and +far to be "farthest" in 2d, so we offset
        // the camera's translation by far and use a right handed coordinate system
        let far = 1000.0;
        Camera2dBundle {
            camera: Camera {
                name: Some(base::camera::CAMERA_2D.to_string()),
                ..Default::default()
            },
            orthographic_projection: OrthographicProjection {
                far,
                ..Default::default()
            },
            visible_entities: Default::default(),
            transform: Transform::from_xyz(0.0, 0.0, far - 0.1),
            global_transform: Default::default(),
        }
    }
}

/// A component bundle for "orthographic camera" entities
///
/// Uses a normalized orthographic projection.
///
/// Use this for CAD-like 3D views, isometric games,
/// or 2D games that should scale with the window.
#[derive(Bundle)]
pub struct CameraOrthoBundle {
    pub camera: Camera,
    pub orthographic_projection: ScaledOrthographicProjection,
    pub visible_entities: VisibleEntities,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for CameraOrthoBundle {
    fn default() -> Self {
        CameraOrthoBundle {
            camera: Camera {
                name: Some(base::camera::CAMERA_3D.to_string()),
                ..Default::default()
            },
            orthographic_projection: ScaledOrthographicProjection::default(),
            visible_entities: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}
