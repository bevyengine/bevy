//! Utilities for converting from glTF's [standard coordinate system](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#coordinate-system-and-units)
//! to Bevy's.
use serde::{Deserialize, Serialize};

use bevy_math::{Mat4, Quat, Vec3};
use bevy_transform::components::Transform;

/*
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
*/

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
    /// XXX TODO: Update documentation.
    ///
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
    pub rotate_scene: bool,

    /// XXX TODO: Documentation.
    pub rotate_nodes: bool,

    /// XXX TODO: Update documentation.
    ///
    /// If true, convert mesh assets and skinned mesh bind poses.
    ///
    /// This option only changes mesh assets and the transforms of entities that
    /// instance meshes through [`Mesh3d`](bevy_mesh::Mesh3d).
    pub rotate_meshes: bool,
}

#[derive(Copy, Clone)]
pub(crate) struct Conversion {
    local: Quat,
    inverse_parent: Quat,
}

impl Conversion {
    pub(crate) const GLTF_TO_BEVY: Quat = Quat::from_xyzw(0.0, 1.0, 0.0, 0.0);

    pub(crate) fn from_local_and_parent(local: Quat, parent: Quat) -> Self {
        Self {
            local,
            inverse_parent: parent.inverse(),
        }
    }

    pub(crate) fn from_parent(parent: Quat) -> Self {
        Self {
            local: Quat::IDENTITY,
            inverse_parent: parent.inverse(),
        }
    }

    pub(crate) fn translation(&self, t: Vec3) -> Vec3 {
        self.inverse_parent * t
    }

    pub(crate) fn rotation(&self, r: Quat) -> Quat {
        self.inverse_parent * r * self.local
    }

    pub(crate) fn scale(&self, s: Vec3) -> Vec3 {
        // XXX TODO
        s
    }

    pub(crate) fn transform(&self, t: Transform) -> Transform {
        Transform::from_translation(self.translation(t.translation))
            .with_rotation(self.rotation(t.rotation))
            .with_scale(self.scale(t.scale))
    }

    pub(crate) fn mat4(&self, m: Mat4) -> Mat4 {
        // XXX TODO: Consider more efficient alternatives.
        let inverse_parent_matrix = Mat4::from_quat(self.inverse_parent);
        let local_matrix = Mat4::from_quat(self.local);

        inverse_parent_matrix * m * local_matrix
    }

    pub(crate) fn inverse_mat4(&self, m: Mat4) -> Mat4 {
        // XXX TODO:
        self.mat4(m)
    }
}
