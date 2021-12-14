use crate::{
    camera::{
        Camera, CameraPlugin, DepthCalculation, OrthographicProjection, PerspectiveProjection,
        ScalingMode,
    },
    primitives::Frustum,
    view::VisibleEntities,
};
use bevy_ecs::bundle::Bundle;
use bevy_math::Vec3;
use bevy_transform::components::{GlobalTransform, Transform};

use super::CameraProjection;

/// Component bundle for camera entities with perspective projection
///
/// Use this for 3D rendering.
#[derive(Bundle)]
pub struct PerspectiveCameraBundle {
    pub camera: Camera,
    pub perspective_projection: PerspectiveProjection,
    pub visible_entities: VisibleEntities,
    pub frustum: Frustum,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl PerspectiveCameraBundle {
    pub fn new_3d() -> Self {
        Default::default()
    }

    pub fn with_name(name: &str) -> Self {
        let perspective_projection = PerspectiveProjection::default();
        let view_projection = perspective_projection.get_projection_matrix();
        let frustum = Frustum::from_view_projection(
            &view_projection,
            &Vec3::ZERO,
            &Vec3::Z,
            perspective_projection.far(),
        );
        PerspectiveCameraBundle {
            camera: Camera {
                name: Some(name.to_string()),
                near: perspective_projection.near,
                far: perspective_projection.far,
                ..Default::default()
            },
            perspective_projection,
            visible_entities: VisibleEntities::default(),
            frustum,
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

impl Default for PerspectiveCameraBundle {
    fn default() -> Self {
        PerspectiveCameraBundle::with_name(CameraPlugin::CAMERA_3D)
    }
}

pub struct PerspectiveCameraBundleBuilder {
    name: Option<String>,
    perspective_projection: PerspectiveProjection,
    transform: Transform,
}

impl PerspectiveCameraBundleBuilder {
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn perspective_projection(mut self, perspective_projection: PerspectiveProjection) -> Self {
        self.perspective_projection = perspective_projection;
        self
    }

    pub fn transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    pub fn build(self) -> PerspectiveCameraBundle {
        let view_projection = self.perspective_projection.get_projection_matrix();
        let frustum = Frustum::from_view_projection(
            &view_projection,
            &Vec3::ZERO,
            &Vec3::Z,
            self.perspective_projection.far(),
        );
        PerspectiveCameraBundle {
            camera: Camera {
                name: self.name,
                near: self.perspective_projection.near,
                far: self.perspective_projection.far,
                ..Default::default()
            },
            perspective_projection: self.perspective_projection,
            visible_entities: VisibleEntities::default(),
            frustum,
            transform: self.transform,
            global_transform: Default::default(),
        }
    }
}

impl Default for PerspectiveCameraBundleBuilder {
    fn default() -> Self {
        Self {
            name: Some(CameraPlugin::CAMERA_3D.to_string()),
            perspective_projection: Default::default(),
            transform: Default::default(),
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
    pub frustum: Frustum,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl OrthographicCameraBundle {
    pub fn new_2d() -> Self {
        // we want 0 to be "closest" and +far to be "farthest" in 2d, so we offset
        // the camera's translation by far and use a right handed coordinate system
        let far = 1000.0;
        let orthographic_projection = OrthographicProjection {
            far,
            depth_calculation: DepthCalculation::ZDifference,
            ..Default::default()
        };
        let transform = Transform::from_xyz(0.0, 0.0, far - 0.1);
        let view_projection =
            orthographic_projection.get_projection_matrix() * transform.compute_matrix().inverse();
        let frustum = Frustum::from_view_projection(
            &view_projection,
            &transform.translation,
            &transform.back(),
            orthographic_projection.far(),
        );
        OrthographicCameraBundle {
            camera: Camera {
                name: Some(CameraPlugin::CAMERA_2D.to_string()),
                near: orthographic_projection.near,
                far: orthographic_projection.far,
                ..Default::default()
            },
            orthographic_projection,
            visible_entities: VisibleEntities::default(),
            frustum,
            transform,
            global_transform: Default::default(),
        }
    }

    pub fn new_3d() -> Self {
        let orthographic_projection = OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical,
            depth_calculation: DepthCalculation::Distance,
            ..Default::default()
        };
        let view_projection = orthographic_projection.get_projection_matrix();
        let frustum = Frustum::from_view_projection(
            &view_projection,
            &Vec3::ZERO,
            &Vec3::Z,
            orthographic_projection.far(),
        );
        OrthographicCameraBundle {
            camera: Camera {
                name: Some(CameraPlugin::CAMERA_3D.to_string()),
                near: orthographic_projection.near,
                far: orthographic_projection.far,
                ..Default::default()
            },
            orthographic_projection,
            visible_entities: VisibleEntities::default(),
            frustum,
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }

    pub fn with_name(name: &str) -> Self {
        let orthographic_projection = OrthographicProjection::default();
        let view_projection = orthographic_projection.get_projection_matrix();
        let frustum = Frustum::from_view_projection(
            &view_projection,
            &Vec3::ZERO,
            &Vec3::Z,
            orthographic_projection.far(),
        );
        OrthographicCameraBundle {
            camera: Camera {
                name: Some(name.to_string()),
                near: orthographic_projection.near,
                far: orthographic_projection.far,
                ..Default::default()
            },
            orthographic_projection,
            visible_entities: VisibleEntities::default(),
            frustum,
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}
