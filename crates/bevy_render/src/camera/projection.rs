use std::marker::PhantomData;

use bevy_app::{App, CoreStage, Plugin, StartupStage};
use bevy_ecs::{prelude::*, reflect::ReflectComponent};
use bevy_math::Mat4;
use bevy_reflect::{
    std_traits::ReflectDefault, FromReflect, GetTypeRegistration, Reflect, ReflectDeserialize,
    ReflectSerialize,
};
use bevy_window::ModifiesWindows;
use serde::{Deserialize, Serialize};

/// Adds [`Camera`](crate::camera::Camera) driver systems for a given projection type.
pub struct CameraProjectionPlugin<T: CameraProjection>(PhantomData<T>);

impl<T: CameraProjection> Default for CameraProjectionPlugin<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

#[derive(SystemLabel, Clone, Eq, PartialEq, Hash, Debug)]
pub struct CameraUpdateSystem;

impl<T: CameraProjection + Component + GetTypeRegistration> Plugin for CameraProjectionPlugin<T> {
    fn build(&self, app: &mut App) {
        app.register_type::<T>()
            .add_startup_system_to_stage(
                StartupStage::PostStartup,
                crate::camera::camera_system::<T>,
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                crate::camera::camera_system::<T>
                    .label(CameraUpdateSystem)
                    .after(ModifiesWindows),
            );
    }
}

/// Trait to control the projection matrix of a camera.
///
/// Components implementing this trait are automatically polled for changes, and used
/// to recompute the camera projection matrix of the [`Camera`] component attached to
/// the same entity as the component implementing this trait.
///
/// [`Camera`]: crate::camera::Camera
pub trait CameraProjection {
    fn get_projection_matrix(&self) -> Mat4;
    fn update(&mut self, width: f32, height: f32);
    fn far(&self) -> f32;
}

/// A configurable [`CameraProjection`] that can select its projection type at runtime.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default)]
pub enum Projection {
    Perspective(PerspectiveProjection),
    Orthographic(OrthographicProjection),
}

impl From<PerspectiveProjection> for Projection {
    fn from(p: PerspectiveProjection) -> Self {
        Self::Perspective(p)
    }
}

impl From<OrthographicProjection> for Projection {
    fn from(p: OrthographicProjection) -> Self {
        Self::Orthographic(p)
    }
}

impl CameraProjection for Projection {
    fn get_projection_matrix(&self) -> Mat4 {
        match self {
            Projection::Perspective(projection) => projection.get_projection_matrix(),
            Projection::Orthographic(projection) => projection.get_projection_matrix(),
        }
    }

    fn update(&mut self, width: f32, height: f32) {
        match self {
            Projection::Perspective(projection) => projection.update(width, height),
            Projection::Orthographic(projection) => projection.update(width, height),
        }
    }

    fn far(&self) -> f32 {
        match self {
            Projection::Perspective(projection) => projection.far(),
            Projection::Orthographic(projection) => projection.far(),
        }
    }
}

impl Default for Projection {
    fn default() -> Self {
        Projection::Perspective(Default::default())
    }
}

#[derive(Component, Debug, Clone, Reflect, FromReflect)]
#[reflect(Component, Default)]
pub struct PerspectiveProjection {
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl CameraProjection for PerspectiveProjection {
    fn get_projection_matrix(&self) -> Mat4 {
        Mat4::perspective_infinite_reverse_rh(self.fov, self.aspect_ratio, self.near)
    }

    fn update(&mut self, width: f32, height: f32) {
        self.aspect_ratio = width / height;
    }

    fn far(&self) -> f32 {
        self.far
    }
}

impl Default for PerspectiveProjection {
    fn default() -> Self {
        PerspectiveProjection {
            fov: std::f32::consts::PI / 4.0,
            near: 0.1,
            far: 1000.0,
            aspect_ratio: 1.0,
        }
    }
}

// TODO: make this a component instead of a property
#[derive(Debug, Clone, Reflect, FromReflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub enum WindowOrigin {
    Center,
    BottomLeft,
}

#[derive(Debug, Clone, Reflect, FromReflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub enum ScalingMode {
    /// Manually specify left/right/top/bottom values.
    /// Ignore window resizing; the image will stretch.
    None,
    /// Match the window size. 1 world unit = 1 pixel.
    WindowSize,
    /// Use minimal possible viewport size while keeping the aspect ratio.
    /// Arguments are in world units.
    Auto { min_width: f32, min_height: f32 },
    /// Keep vertical axis constant; resize horizontal with aspect ratio.
    /// The argument is the desired height of the viewport in world units.
    FixedVertical(f32),
    /// Keep horizontal axis constant; resize vertical with aspect ratio.
    /// The argument is the desired width of the viewport in world units.
    FixedHorizontal(f32),
}

#[derive(Component, Debug, Clone, Reflect, FromReflect)]
#[reflect(Component, Default)]
pub struct OrthographicProjection {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
    pub window_origin: WindowOrigin,
    pub scaling_mode: ScalingMode,
    pub scale: f32,
}

impl CameraProjection for OrthographicProjection {
    fn get_projection_matrix(&self) -> Mat4 {
        Mat4::orthographic_rh(
            self.left * self.scale,
            self.right * self.scale,
            self.bottom * self.scale,
            self.top * self.scale,
            // NOTE: near and far are swapped to invert the depth range from [0,1] to [1,0]
            // This is for interoperability with pipelines using infinite reverse perspective projections.
            self.far,
            self.near,
        )
    }

    fn update(&mut self, width: f32, height: f32) {
        let (viewport_width, viewport_height) = match self.scaling_mode {
            ScalingMode::WindowSize => (width, height),
            ScalingMode::Auto {
                min_width,
                min_height,
            } => {
                if width * min_height > min_width * height {
                    (width * min_height / height, min_height)
                } else {
                    (min_width, height * min_width / width)
                }
            }
            ScalingMode::FixedVertical(viewport_height) => {
                (width * viewport_height / height, viewport_height)
            }
            ScalingMode::FixedHorizontal(viewport_width) => {
                (viewport_width, height * viewport_width / width)
            }
            ScalingMode::None => return,
        };

        match self.window_origin {
            WindowOrigin::Center => {
                let half_width = viewport_width / 2.0;
                let half_height = viewport_height / 2.0;
                self.left = -half_width;
                self.bottom = -half_height;
                self.right = half_width;
                self.top = half_height;

                if let ScalingMode::WindowSize = self.scaling_mode {
                    if self.scale == 1.0 {
                        self.left = self.left.floor();
                        self.bottom = self.bottom.floor();
                        self.right = self.right.floor();
                        self.top = self.top.floor();
                    }
                }
            }
            WindowOrigin::BottomLeft => {
                self.left = 0.0;
                self.bottom = 0.0;
                self.right = viewport_width;
                self.top = viewport_height;
            }
        }
    }

    fn far(&self) -> f32 {
        self.far
    }
}

impl Default for OrthographicProjection {
    fn default() -> Self {
        OrthographicProjection {
            left: -1.0,
            right: 1.0,
            bottom: -1.0,
            top: 1.0,
            near: 0.0,
            far: 1000.0,
            window_origin: WindowOrigin::Center,
            scaling_mode: ScalingMode::WindowSize,
            scale: 1.0,
        }
    }
}
