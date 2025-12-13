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

/// Options for converting scenes and assets from glTF's [standard coordinate system](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#coordinate-system-and-units)
/// (+Z forward) to Bevy's coordinate system (-Z forward).
///
/// The exact coordinate system conversion is as follows:
/// - glTF:
///   - forward: +Z
///   - up: +Y
///   - right: -X
/// - Bevy:
///   - forward: -Z
///   - up: +Y
///   - right: +X
///
/// Note that some glTF files may not follow the glTF standard.
///
/// If your glTF scene is +Z forward and you want it converted to match Bevy's
/// `Transform::forward`, enable the `rotate_scene_entity` option. If you also want `Mesh`
/// assets to be converted, enable the `rotate_meshes` option.
///
/// Cameras and lights in glTF files are an exception - they already use Bevy's
/// coordinate system. This means cameras and lights will match
/// `Transform::forward` even if conversion is disabled.
#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize)]
pub struct GltfConvertCoordinates {
    /// If true, convert scenes by rotating the top-level transform of the scene entity.
    ///
    /// This will ensure that [`Transform::forward`] of the "root" entity (the one with [`SceneInstance`](bevy_scene::SceneInstance))
    /// aligns with the "forward" of the glTF scene.
    ///
    /// The glTF loader creates an entity for each glTF scene. Entities are
    /// then created for each node within the glTF scene. Nodes without a
    /// parent in the glTF scene become children of the scene entity.
    ///
    /// This option only changes the transform of the scene entity. It does not
    /// directly change the transforms of node entities - it only changes them
    /// indirectly through transform inheritance.
    pub rotate_scene_entity: bool,

    /// If true, convert mesh assets. This includes skinned mesh bind poses.
    ///
    /// This option only changes mesh assets and the transforms of entities that
    /// instance meshes. It does not change the transforms of entities that
    /// correspond to glTF nodes.
    pub rotate_meshes: bool,
}

impl GltfConvertCoordinates {
    const CONVERSION_TRANSFORM: Transform =
        Transform::from_rotation(Quat::from_xyzw(0.0, 1.0, 0.0, 0.0));

    fn conversion_mat4() -> Mat4 {
        Mat4::from_scale(Vec3::new(-1.0, 1.0, -1.0))
    }

    pub(crate) fn scene_conversion_transform(&self) -> Transform {
        if self.rotate_scene_entity {
            Self::CONVERSION_TRANSFORM
        } else {
            Transform::IDENTITY
        }
    }

    pub(crate) fn mesh_conversion_transform(&self) -> Transform {
        if self.rotate_meshes {
            Self::CONVERSION_TRANSFORM
        } else {
            Transform::IDENTITY
        }
    }

    pub(crate) fn mesh_conversion_transform_inverse(&self) -> Transform {
        // We magically know that the transform is its own inverse. We still
        // make a distinction at the interface level in case that changes.
        self.mesh_conversion_transform()
    }

    pub(crate) fn mesh_conversion_mat4(&self) -> Mat4 {
        if self.rotate_meshes {
            Self::conversion_mat4()
        } else {
            Mat4::IDENTITY
        }
    }
}
