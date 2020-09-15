use bevy_math::{Mat3, Mat4, Quat, Vec3};
use bevy_property::Properties;
use std::fmt;

#[derive(Debug, PartialEq, Clone, Copy, Properties)]
pub struct GlobalTransform {
    value: Mat4,
}

impl GlobalTransform {
    #[inline(always)]
    pub fn new(value: Mat4) -> Self {
        GlobalTransform { value }
    }

    #[inline(always)]
    pub fn identity() -> Self {
        GlobalTransform {
            value: Mat4::identity(),
        }
    }

    pub fn from_translation(translation: Vec3) -> Self {
        GlobalTransform::new(Mat4::from_translation(translation))
    }

    pub fn from_rotation(rotation: Quat) -> Self {
        GlobalTransform::new(Mat4::from_quat(rotation))
    }

    // TODO: make sure scale is positive
    pub fn from_scale(scale: Vec3) -> Self {
        GlobalTransform::new(Mat4::from_scale(scale))
    }

    pub fn with_translation(mut self, translation: Vec3) -> Self {
        self.translate(translation);
        self
    }

    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotate(rotation);
        self
    }

    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.apply_scale(scale);
        self
    }

    pub fn value(&self) -> &Mat4 {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut Mat4 {
        &mut self.value
    }

    pub fn translation(&self) -> Vec3 {
        Vec3::from(self.value.w_axis().truncate())
    }

    pub fn rotation(&self) -> Quat {
        let scale = self.scale();

        Quat::from_rotation_mat3(&Mat3::from_cols(
            Vec3::from(self.value.x_axis().truncate()) / scale.x(),
            Vec3::from(self.value.y_axis().truncate()) / scale.y(),
            Vec3::from(self.value.z_axis().truncate()) / scale.z(),
        ))
    }

    pub fn scale(&self) -> Vec3 {
        Vec3::new(
            self.value.x_axis().truncate().length(),
            self.value.y_axis().truncate().length(),
            self.value.z_axis().truncate().length(),
        )
    }

    pub fn set_translation(&mut self, translation: Vec3) {
        *self.value.w_axis_mut() = translation.extend(1.0);
    }

    pub fn set_rotation(&mut self, rotation: Quat) {
        let rotation = rotation * self.rotation().conjugate();
        rotation.normalize();
        self.value = Mat4::from_quat(rotation) * self.value;
    }

    pub fn set_scale(&mut self, scale: Vec3) {
        let scale = scale / self.scale();
        self.value = Mat4::from_scale(scale) * self.value;
    }

    pub fn translate(&mut self, translation: Vec3) {
        *self.value.w_axis_mut() += translation.extend(0.0);
    }

    pub fn rotate(&mut self, rotation: Quat) {
        self.value = Mat4::from_quat(rotation) * self.value;
    }

    pub fn apply_scale(&mut self, scale: Vec3) {
        self.value = Mat4::from_scale(scale) * self.value;
    }
}

impl Default for GlobalTransform {
    fn default() -> Self {
        Self::identity()
    }
}

impl fmt::Display for GlobalTransform {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}
