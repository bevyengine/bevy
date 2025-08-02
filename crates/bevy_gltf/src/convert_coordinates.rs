use serde::{Deserialize, Serialize};

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
    ///
    /// See <https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#coordinate-system-and-units>
    fn convert_coordinates(self) -> Self;
}

pub(crate) trait ConvertInverseCoordinates {
    fn convert_inverse_coordinates(self) -> Self;
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
        // Solution of q' = r q r*
        [-self[0], self[1], -self[2], self[3]]
    }
}

// XXX TODO: Documentation.
impl ConvertInverseCoordinates for Mat4 {
    fn convert_inverse_coordinates(self) -> Self {
        self * Mat4::from_scale(Vec3::new(-1.0, 1.0, -1.0))
    }
}

// XXX TODO: Documentation.
#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize)]
pub struct GltfConvertCoordinates {
    pub scene: bool,
    pub meshes: bool,
}

impl GltfConvertCoordinates {
    const TRANSFORM_BEVY_FROM_GLTF: Transform =
        Transform::from_rotation(Quat::from_xyzw(0.0, 1.0, 0.0, 0.0));

    pub(crate) fn scene_conversion_transform(&self) -> Transform {
        if self.scene {
            Self::TRANSFORM_BEVY_FROM_GLTF
        } else {
            Transform::IDENTITY
        }
    }

    pub(crate) fn mesh_conversion_transform(&self) -> Transform {
        if self.meshes {
            Self::TRANSFORM_BEVY_FROM_GLTF
        } else {
            Transform::IDENTITY
        }
    }
}
