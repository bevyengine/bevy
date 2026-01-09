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
/// _CAUTION: This is an experimental feature. Behavior may change in future versions._
///
/// The exact coordinate system conversion is as follows:
///
/// - glTF:
///   - forward: +Z
///   - up: +Y
///   - right: -X
/// - Bevy:
///   - forward: -Z
///   - up: +Y
///   - right: +X
///
/// Cameras and lights are an exception - they already use Bevy's coordinate
/// system. This means cameras and lights will match Bevy's forward even if
/// conversion is disabled.
///
/// If a glTF file uses the standard coordinate system, then the conversion
/// options will behave like so:
///
/// - `rotate_scene_entity` will make the glTF's scene forward align with the [`Transform::forward`]
///   of the entity with the [`SceneInstance`](bevy_scene::SceneInstance) component.
/// - `rotate_meshes` will do the same for entities with a `Mesh3d` component.
///
/// Other entities in the scene are not converted, so their forward may not
/// match `Transform::forward`. In particular, the entities that correspond to
/// glTF nodes are not converted.
#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize)]
pub struct GltfConvertCoordinates {
    /// If true, convert scenes by rotating the top-level transform of the scene entity.
    /// This will ensure that [`Transform::forward`] of the "root" entity (the one with [`SceneInstance`](bevy_scene::SceneInstance))
    /// aligns with the "forward" of the glTF scene.
    ///
    /// The scene entity is created by the glTF loader. Its parent is the entity
    /// with the `SceneInstance` component, and its children are the root nodes
    /// of the glTF scene.
    ///
    /// This option only changes the transform of the scene entity. It does not
    /// directly change the transforms of node entities - it only changes them
    /// indirectly through transform inheritance.
    pub rotate_scene_entity: bool,

    /// If true, convert mesh assets and skinned mesh bind poses.
    ///
    /// This option only changes mesh assets and the transforms of entities that
    /// instance meshes through [`Mesh3d`](bevy_mesh::Mesh3d).
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
