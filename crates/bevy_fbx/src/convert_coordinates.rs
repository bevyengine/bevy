use bevy_math::{Quat, Vec3};
use bevy_transform::components::Transform;

/// Trait for converting FBX coordinates to Bevy's coordinate system.
pub(crate) trait ConvertCoordinates {
    /// Converts FBX coordinates to Bevy's coordinate system.
    ///
    /// FBX default coordinate system (can vary):
    /// - forward: +Z
    /// - up: +Y  
    /// - right: +X
    ///
    /// Bevy coordinate system:
    /// - forward: -Z
    /// - up: +Y
    /// - right: +X
    fn convert_coordinates(self) -> Self;
}

impl ConvertCoordinates for Vec3 {
    fn convert_coordinates(self) -> Self {
        // FBX to Bevy: negate Z to flip forward direction
        Vec3::new(self.x, self.y, -self.z)
    }
}

impl ConvertCoordinates for [f32; 3] {
    fn convert_coordinates(self) -> Self {
        [self[0], self[1], -self[2]]
    }
}

impl ConvertCoordinates for Quat {
    fn convert_coordinates(self) -> Self {
        // Quaternion conversion for coordinate system change
        // Flip the Z component to handle the coordinate system change
        Quat::from_xyzw(self.x, self.y, -self.z, self.w)
    }
}

impl ConvertCoordinates for Transform {
    fn convert_coordinates(mut self) -> Self {
        self.translation = self.translation.convert_coordinates();
        self.rotation = self.rotation.convert_coordinates();
        // Scale remains the same
        self
    }
}
