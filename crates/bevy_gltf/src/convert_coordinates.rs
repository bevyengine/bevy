use std::f32::consts::PI;

use bevy_math::{Mat4, Quat, Vec3};
use bevy_transform::components::Transform;

pub(crate) trait ConvertCoordinates {
    /// Converts the glTF coordinates to Bevy's coordinate system.
    /// - glTF:
    ///   - forward: Z
    ///   - up: Y
    ///   - right: -X
    /// - Bevy:
    ///   - forward: -Z
    ///   - up: Y
    ///   - right: X
    /// See <https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#coordinate-system-and-units>
    fn convert_coordinates(self) -> Self;
}

pub(crate) trait ConvertCameraCoordinates {
    /// Like `convert_coordinates`, but uses the following for the lens rotation:
    /// - forward: -Z
    /// - up: Y
    /// - right: X
    /// See <https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#view-matrix>
    fn convert_camera_coordinates(self) -> Self;
}

impl ConvertCoordinates for Vec3 {
    fn convert_coordinates(self) -> Self {
        Vec3::new(-self.x, self.y, -self.z)
    }
}

impl ConvertCoordinates for [f32; 3] {
    fn convert_coordinates(self) -> Self {
        [-self[0], self[1], -self[2]]
    }
}

impl ConvertCoordinates for [f32; 4] {
    fn convert_coordinates(self) -> Self {
        [-self[0], self[1], -self[2], self[3]]
    }
}

impl ConvertCoordinates for Quat {
    fn convert_coordinates(self) -> Self {
        Quat::from_array([-self.x, self.y, -self.z, self.w])
    }
}

impl ConvertCoordinates for Mat4 {
    fn convert_coordinates(self) -> Self {
        let coordinate_conversion = Mat4::from_scale(Vec3::new(-1.0, 1.0, -1.0));
        // the inverse is the same as the original
        let coordinate_conversion_inv = coordinate_conversion;
        coordinate_conversion * self * coordinate_conversion_inv
    }
}

impl ConvertCoordinates for Transform {
    fn convert_coordinates(mut self) -> Self {
        self.translation = self.translation.convert_coordinates();
        self.rotation = self.rotation.convert_coordinates();
        self
    }
}

impl ConvertCameraCoordinates for Transform {
    fn convert_camera_coordinates(mut self) -> Self {
        self.translation = self.translation.convert_coordinates();
        self.rotate_y(PI);
        self
    }
}
