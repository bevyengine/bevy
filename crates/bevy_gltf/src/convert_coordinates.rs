//! Utilities for converting from glTF's [standard coordinate system](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#coordinate-system-and-units)
//! to Bevy's.
use serde::{Deserialize, Serialize};

use bevy_math::{Mat4, Quat, Vec3};
use bevy_transform::components::Transform;

pub(crate) trait ConvertCoordinates {
    /// Converts from glTF coordinates to Bevy's coordinate system. See
    /// [`GltfConvertCoordinates`] for an explanation of the conversion.
    fn convert_coordinates(self) -> Self;
}

// XXX TODO: Documentation.
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

/// Options for converting scenes and assets from glTF's [standard coordinate system](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#coordinate-system-and-units)
/// to Bevy's.
///
/// The exact coordinate system conversion is as follows:
/// - glTF:
///   - forward: Z
///   - up: Y
///   - right: -X
/// - Bevy:
///   - forward: -Z
///   - up: Y
///   - right: X
///
/// Note that some glTF files may not follow the glTF standard.
#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize)]
pub struct GltfConvertCoordinates {
    /// If true, convert scenes via the transform of the root entity.
    ///
    /// The glTF loader works by creating an entity for each glTF scene, and an
    /// entity for each glTF node within the scene. If a node doesn't have a
    /// parent then its entity is parented to the scene entity.
    ///
    /// This option only changes the transform of the scene entity. It does not
    /// directly change the transforms of node entities - it only changes them
    /// indirectly through transform inheritance.
    pub scenes: bool,

    /// If true, convert mesh assets. This includes skinned mesh bind poses.
    ///
    /// This option only changes mesh assets and the transforms of entities that
    /// instance meshes. It does not change the transforms of entities that
    /// correspond to glTF nodes.
    pub meshes: bool,
}

impl GltfConvertCoordinates {
    const TRANSFORM_BEVY_FROM_GLTF: Transform =
        Transform::from_rotation(Quat::from_xyzw(0.0, 1.0, 0.0, 0.0));

    pub(crate) fn scene_conversion_transform(&self) -> Transform {
        if self.scenes {
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
