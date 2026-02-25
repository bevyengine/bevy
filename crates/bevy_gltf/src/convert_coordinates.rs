//! Utilities for converting from glTF's [standard coordinate system](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#coordinate-system-and-units)
//! to Bevy's.
use bevy_mesh::{Mesh, MeshVertexAttribute, VertexAttributeValues, VertexFormat};
use gltf::Node;
use serde::{Deserialize, Serialize};

use bevy_math::{Quat, Vec3, Vec4};
use bevy_transform::components::Transform;
use thiserror::Error;

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

impl GltfConvertCoordinates {
    const GLTF_TO_BEVY: Quat = Quat::from_xyzw(0.0, 1.0, 0.0, 0.0);

    fn conversion_rotation(convert: bool) -> Quat {
        if convert {
            Self::GLTF_TO_BEVY
        } else {
            Quat::IDENTITY
        }
    }

    pub(crate) fn node_rotation(&self, node: &Node) -> Quat {
        Self::conversion_rotation(
            self.rotate_nodes && node.camera().is_none() && node.light().is_none(),
        )
    }

    pub(crate) fn scene_rotation(&self) -> Quat {
        Self::conversion_rotation(self.rotate_scene)
    }

    pub(crate) fn mesh_rotation(&self) -> Quat {
        Self::conversion_rotation(self.rotate_meshes)
    }
}

#[derive(Error, Debug)]
pub(crate) enum CoordinateConversionAttributeError {
    #[error("Cannot apply coordinate conversion to attribute {0} - unsupported format {1:?}")]
    UnsupportedFormat(&'static str, VertexFormat),
}

pub(crate) fn attribute_coordinate_conversion(
    attribute: MeshVertexAttribute,
    values: VertexAttributeValues,
    rotation: Quat,
) -> Result<VertexAttributeValues, CoordinateConversionAttributeError> {
    match attribute {
        Mesh::ATTRIBUTE_POSITION | Mesh::ATTRIBUTE_NORMAL | Mesh::ATTRIBUTE_TANGENT => match values
        {
            VertexAttributeValues::Float32x3(mut values) => {
                for value in &mut values {
                    *value = (rotation * Vec3::from_array(*value)).to_array();
                }
                Ok(VertexAttributeValues::Float32x3(values))
            }

            VertexAttributeValues::Float32x4(mut values) => {
                for value in &mut values {
                    *value = (rotation * Vec4::from_array(*value).truncate())
                        .extend(value[3])
                        .to_array();
                }
                Ok(VertexAttributeValues::Float32x4(values))
            }

            _ => Err(CoordinateConversionAttributeError::UnsupportedFormat(
                attribute.name,
                VertexFormat::from(&values),
            )),
        },
        _ => Ok(values),
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct Conversion {
    local: Quat,
    parent: Quat,
}

impl Conversion {
    pub(crate) fn from_local_and_parent(local: Quat, parent: Quat) -> Self {
        Self { local, parent }
    }

    pub(crate) fn local(&self) -> Quat {
        self.local
    }

    pub(crate) fn translation(&self, t: Vec3) -> Vec3 {
        self.parent.inverse() * t
    }

    pub(crate) fn rotation(&self, r: Quat) -> Quat {
        self.parent.inverse() * r * self.local
    }

    pub(crate) fn scale(&self, s: Vec3) -> Vec3 {
        // XXX TODO
        //self.local.inverse() * s
        s
    }

    pub(crate) fn transform(&self, t: Transform) -> Transform {
        Transform::from_translation(self.translation(t.translation))
            .with_rotation(self.rotation(t.rotation))
            .with_scale(self.scale(t.scale))
    }
}
