use bevy_math::{Mat3, Mat4, Quat, Vec3};
use bevy_property::Properties;
use std::fmt;

#[derive(Debug, PartialEq, Clone, Copy, Properties)]
pub struct GlobalTransform {
    pub value: Mat4,
}

impl GlobalTransform {
    #[inline(always)]
    pub fn identity() -> Self {
        GlobalTransform {
            value: Mat4::identity(),
        }
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
