//! Utilities for converting from glTF's [standard coordinate system](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#coordinate-system-and-units)
//! to Bevy's.
use bevy_mesh::{Mesh, MeshVertexAttribute, VertexAttributeValues, VertexFormat};
use gltf::Node;
use serde::{Deserialize, Serialize};

use bevy_math::{vec3, Quat, Vec3, Vec4};
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

    pub(crate) fn node_hierarchy_conversion(
        &self,
        node: &Node,
        parent_node: Option<&Node>,
    ) -> HierarchyConversion {
        let parent_conversion = if let Some(parent_node) = parent_node {
            self.node_rotation(parent_node)
        } else {
            self.scene_rotation()
        };

        let local_conversion = self.node_rotation(node);

        HierarchyConversion::from_local_and_parent(local_conversion, parent_conversion)
    }

    pub(crate) fn mesh_hierarchy_conversion(&self, node: &Node) -> HierarchyConversion {
        HierarchyConversion::from_local_and_parent(self.mesh_rotation(), self.node_rotation(node))
    }

    pub(crate) fn mesh_vertex_rotation(&self) -> Quat {
        self.mesh_rotation().inverse()
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::X),
            1 => Some(Self::Y),
            2 => Some(Self::Z),
            _ => None,
        }
    }

    fn from_index_unchecked(index: usize) -> Self {
        Self::from_index(index).unwrap()
    }

    fn index(&self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
}

impl From<Axis> for Vec3 {
    fn from(value: Axis) -> Self {
        match value {
            Axis::X => vec3(1.0, 0.0, 0.0),
            Axis::Y => vec3(0.0, 1.0, 0.0),
            Axis::Z => vec3(0.0, 0.0, 1.0),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Sign {
    Positive,
    Negative,
}

impl Sign {
    fn flip(&self) -> Sign {
        match self {
            Self::Positive => Self::Negative,
            Self::Negative => Self::Positive,
        }
    }

    fn is_positive(&self) -> bool {
        *self == Self::Positive
    }

    fn is_negative(&self) -> bool {
        *self == Self::Negative
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct SignedAxis {
    pub(crate) sign: Sign,
    pub(crate) axis: Axis,
}

impl SignedAxis {
    const POSITIVE_X: Self = Self {
        sign: Sign::Positive,
        axis: Axis::X,
    };
    const POSITIVE_Y: Self = Self {
        sign: Sign::Positive,
        axis: Axis::Y,
    };
    const POSITIVE_Z: Self = Self {
        sign: Sign::Positive,
        axis: Axis::Z,
    };
    const NEGATIVE_X: Self = Self {
        sign: Sign::Negative,
        axis: Axis::X,
    };
    const NEGATIVE_Y: Self = Self {
        sign: Sign::Negative,
        axis: Axis::Y,
    };
    const NEGATIVE_Z: Self = Self {
        sign: Sign::Negative,
        axis: Axis::Z,
    };

    fn from_positive(axis: Axis) -> Self {
        Self {
            sign: Sign::Positive,
            axis,
        }
    }

    fn from_negative(axis: Axis) -> Self {
        Self {
            sign: Sign::Negative,
            axis,
        }
    }

    fn flip(&self) -> SignedAxis {
        Self {
            sign: self.sign.flip(),
            axis: self.axis,
        }
    }

    fn is_positive(&self) -> bool {
        self.sign.is_positive()
    }

    fn is_negative(&self) -> bool {
        self.sign.is_negative()
    }
}

impl From<SignedAxis> for Vec3 {
    fn from(value: SignedAxis) -> Self {
        match value.sign {
            Sign::Positive => Vec3::from(value.axis),
            Sign::Negative => -Vec3::from(value.axis),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct Semantics {
    forward: SignedAxis,
    up: SignedAxis,
    right: SignedAxis,
}

impl Semantics {
    fn from_forward_up(forward: SignedAxis, up: SignedAxis) -> Option<Self> {
        if forward.axis == up.axis {
            return None;
        }

        // right = forward.cross(up)

        let winding = (forward.axis.index() + 1).rem_euclid(3) == up.axis.index();

        let right = SignedAxis {
            sign: match winding ^ forward.sign.is_positive() ^ up.sign.is_positive() {
                true => Sign::Positive,
                false => Sign::Negative,
            },
            axis: Axis::from_index_unchecked(3 - (forward.axis.index() + up.axis.index())),
        };

        Some(Self { forward, up, right })
    }

    fn forward(&self) -> SignedAxis {
        self.forward
    }

    fn back(&self) -> SignedAxis {
        self.forward.flip()
    }

    fn up(&self) -> SignedAxis {
        self.up
    }

    fn down(&self) -> SignedAxis {
        self.up.flip()
    }

    fn right(&self) -> SignedAxis {
        self.right
    }

    fn left(&self) -> SignedAxis {
        self.right.flip()
    }

    const BEVY: Self = Self {
        forward: SignedAxis::NEGATIVE_Z,
        up: SignedAxis::POSITIVE_Y,
        right: SignedAxis::POSITIVE_X,
    };

    const GLTF: Self = Self {
        forward: SignedAxis::POSITIVE_Z,
        up: SignedAxis::POSITIVE_Y,
        right: SignedAxis::NEGATIVE_X,
    };
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct Conversion {
    rotation: Quat,
    swap_axes: [Axis; 3],
    flip_axes: [f32; 3],
}

// Helper for applying a local rotation conversion to nodes in a hierarchy without
// causing them to inherit their parent's conversion.
#[derive(Copy, Clone, Default, Debug)]
pub(crate) struct HierarchyConversion {
    local: Quat,
    parent: Quat,
}

impl HierarchyConversion {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantics() {
        let signed_axes = [
            SignedAxis::POSITIVE_X,
            SignedAxis::POSITIVE_Y,
            SignedAxis::POSITIVE_Z,
            SignedAxis::NEGATIVE_X,
            SignedAxis::NEGATIVE_Y,
            SignedAxis::NEGATIVE_Z,
        ];

        for forward in signed_axes {
            for up in signed_axes {
                if let Some(semantics) = Semantics::from_forward_up(forward, up) {
                    let right = Vec3::from(semantics.right());
                    let cross = Vec3::cross(forward.into(), up.into());

                    assert_eq!(right, cross, "{semantics:?}");
                } else {
                    assert!(forward.axis == up.axis);
                }
            }
        }
    }
}
