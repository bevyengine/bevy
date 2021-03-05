use crate::{
    camera::{
        Camera, DepthCalculation, OrthographicProjection, PerspectiveProjection, ScalingMode,
        VisibleEntities,
    },
    pipeline::RenderPipelines,
    prelude::Visible,
    render_graph::base,
    Draw, Mesh,
};
use base::MainPass;
use bevy_asset::Handle;
use bevy_ecs::bundle::Bundle;
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

/// Component bundle for camera entities with perspective projection
///
/// Use this for 3D rendering.
#[derive(Bundle)]
pub struct PerspectiveCameraBundle {
    pub camera: Camera,
    pub perspective_projection: PerspectiveProjection,
    pub visible_entities: VisibleEntities,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl PerspectiveCameraBundle {
    pub fn new_3d() -> Self {
        Default::default()
    }

    pub fn with_name(name: &str) -> Self {
        PerspectiveCameraBundle {
            camera: Camera {
                name: Some(name.to_string()),
                ..Default::default()
            },
            perspective_projection: Default::default(),
            visible_entities: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

impl Default for PerspectiveCameraBundle {
    fn default() -> Self {
        PerspectiveCameraBundle {
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

/// Component bundle for camera entities with orthographic projection
///
/// Use this for 2D games, isometric games, CAD-like 3D views.
#[derive(Bundle)]
pub struct OrthographicCameraBundle {
    pub camera: Camera,
    pub orthographic_projection: OrthographicProjection,
    pub visible_entities: VisibleEntities,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl OrthographicCameraBundle {
    pub fn new_2d() -> Self {
        // we want 0 to be "closest" and +far to be "farthest" in 2d, so we offset
        // the camera's translation by far and use a right handed coordinate system
        let far = 1000.0;
        OrthographicCameraBundle {
            camera: Camera {
                name: Some(base::camera::CAMERA_2D.to_string()),
                ..Default::default()
            },
            orthographic_projection: OrthographicProjection {
                far,
                depth_calculation: DepthCalculation::ZDifference,
                ..Default::default()
            },
            visible_entities: Default::default(),
            transform: Transform::from_xyz(0.0, 0.0, far - 0.1),
            global_transform: Default::default(),
        }
    }

    pub fn new_3d() -> Self {
        OrthographicCameraBundle {
            camera: Camera {
                name: Some(base::camera::CAMERA_3D.to_string()),
                ..Default::default()
            },
            orthographic_projection: OrthographicProjection {
                scaling_mode: ScalingMode::FixedVertical,
                depth_calculation: DepthCalculation::Distance,
                ..Default::default()
            },
            visible_entities: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }

    pub fn with_name(name: &str) -> Self {
        OrthographicCameraBundle {
            camera: Camera {
                name: Some(name.to_string()),
                ..Default::default()
            },
            orthographic_projection: Default::default(),
            visible_entities: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}
