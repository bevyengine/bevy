use super::DepthCalculation;
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_math::Mat4;
use bevy_reflect::{Reflect, ReflectDeserialize};
use serde::{Deserialize, Serialize};

pub trait CameraProjection {
    fn get_projection_matrix(&self) -> Mat4;
    fn update(&mut self, width: f32, height: f32);
    fn depth_calculation(&self) -> DepthCalculation;
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct PerspectiveProjection {
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl CameraProjection for PerspectiveProjection {
    fn get_projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect_ratio, self.near, self.far)
    }

    fn update(&mut self, width: f32, height: f32) {
        self.aspect_ratio = width / height;
    }

    fn depth_calculation(&self) -> DepthCalculation {
        DepthCalculation::Distance
    }
}

impl Default for PerspectiveProjection {
    fn default() -> Self {
        PerspectiveProjection {
            fov: std::f32::consts::PI / 4.0,
            near: 1.0,
            far: 1000.0,
            aspect_ratio: 1.0,
        }
    }
}

// TODO: make this a component instead of a property
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect_value(Serialize, Deserialize)]
pub enum WindowOrigin {
    Center,
    BottomLeft,
}

#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect_value(Serialize, Deserialize)]
pub enum ScalingMode {
    /// Manually specify left/right/top/bottom values.
    /// Ignore window resizing; the image will stretch.
    None,
    /// Match the window size. 1 world unit = 1 pixel.
    WindowSize,
    /// Keep vertical axis constant; resize horizontal with aspect ratio.
    FixedVertical,
    /// Keep horizontal axis constant; resize vertical with aspect ratio.
    FixedHorizontal,
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
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
    pub depth_calculation: DepthCalculation,
}

impl CameraProjection for OrthographicProjection {
    fn get_projection_matrix(&self) -> Mat4 {
        Mat4::orthographic_rh(
            self.left * self.scale,
            self.right * self.scale,
            self.bottom * self.scale,
            self.top * self.scale,
            self.near,
            self.far,
        )
    }

    fn update(&mut self, width: f32, height: f32) {
        match (&self.scaling_mode, &self.window_origin) {
            (ScalingMode::WindowSize, WindowOrigin::Center) => {
                let half_width = width / 2.0;
                let half_height = height / 2.0;
                self.left = -half_width;
                self.right = half_width;
                self.top = half_height;
                self.bottom = -half_height;
            }
            (ScalingMode::WindowSize, WindowOrigin::BottomLeft) => {
                self.left = 0.0;
                self.right = width;
                self.top = height;
                self.bottom = 0.0;
            }
            (ScalingMode::FixedVertical, WindowOrigin::Center) => {
                let aspect_ratio = width / height;
                self.left = -aspect_ratio;
                self.right = aspect_ratio;
                self.top = 1.0;
                self.bottom = -1.0;
            }
            (ScalingMode::FixedVertical, WindowOrigin::BottomLeft) => {
                let aspect_ratio = width / height;
                self.left = 0.0;
                self.right = aspect_ratio;
                self.top = 1.0;
                self.bottom = 0.0;
            }
            (ScalingMode::FixedHorizontal, WindowOrigin::Center) => {
                let aspect_ratio = height / width;
                self.left = -1.0;
                self.right = 1.0;
                self.top = aspect_ratio;
                self.bottom = -aspect_ratio;
            }
            (ScalingMode::FixedHorizontal, WindowOrigin::BottomLeft) => {
                let aspect_ratio = height / width;
                self.left = 0.0;
                self.right = 1.0;
                self.top = aspect_ratio;
                self.bottom = 0.0;
            }
            (ScalingMode::None, _) => {}
        }
    }

    fn depth_calculation(&self) -> DepthCalculation {
        self.depth_calculation
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
            depth_calculation: DepthCalculation::Distance,
        }
    }
}
