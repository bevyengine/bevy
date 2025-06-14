use bevy_math::{Mat4, Quat, Vec3};

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
    fn convert_coordinates(self) -> Self;
}

impl ConvertCoordinates for Vec3 {
    fn convert_coordinates(mut self) -> Self {
        self.x = -self.x;
        self.z = -self.z;
        self
    }
}

impl ConvertCoordinates for [f32; 3] {
    fn convert_coordinates(mut self) -> Self {
        self[0] = -self[0];
        self[2] = -self[2];
        self
    }
}

impl ConvertCoordinates for [f32; 4] {
    fn convert_coordinates(mut self) -> Self {
        self[0] = -self[0];
        self[2] = -self[2];
        self
    }
}

impl ConvertCoordinates for Quat {
    fn convert_coordinates(mut self) -> Self {
        self.x = -self.x;
        self.z = -self.z;
        self
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
