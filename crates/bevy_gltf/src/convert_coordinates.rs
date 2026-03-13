//! Utilities for converting glTFs between coordinate systems.
use bevy_math::{bounding::Aabb3d, vec3, Dir3, Mat4, Quat, Vec3, Vec4};
use bevy_mesh::{Mesh, MeshVertexAttribute, VertexAttributeValues, VertexFormat};
use bevy_transform::components::Transform;

use core::fmt::Debug;
use gltf::Node;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Options for converting the coordinate systems of glTF scenes during loading.
///
/// _CAUTION: This is an experimental feature. Behavior may change in future versions._
///
/// glTF's [standard coordinate system semantics](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#coordinate-system-and-units)
/// are "+Z forward, +Y up". Bevy's semantics are "-Z forward, +Y up" -
/// almost the same, but *negative* Z forward rather than glTF's *positive* Z
/// forward.
///
/// Without conversion, this means that most glTF scenes would appear backwards
/// in Bevy - or more precisely, the scene's `Transform::forward` would point
/// backwards instead of forwards. To solve this, conversion can be enabled
/// through `GltfConvertCoordinates` in [`GltfPlugin`](crate::GltfPlugin) or
/// [`GltfLoaderSettings`](crate::loader::GltfLoaderSettings).
///
/// Not all glTF scenes follow the standard. It's also common for nodes within
/// the scene to have inconsistent semantics - so the scene might be +Z forward
/// but nodes within the scene are different. `GltfConvertCoordinates` has
/// several options to handle these cases.
///
/// First, conversion can be controlled individually for scenes, nodes and
/// meshes. See [`rotate_scenes`](GltfConvertCoordinates::rotate_scenes),
/// [`rotate_nodes`](GltfConvertCoordinates::rotate_nodes), and [`rotate_meshes`](GltfConvertCoordinates::rotate_meshes).
///
/// Second, the source and target semantics can be overridden. So instead of
/// converting from standard glTF semantics to Bevy semantics, one or both can
/// be overridden with, say, "+X forward, -Z up". See [`GltfConvertSemantics`].
#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize)]
pub struct GltfConvertCoordinates {
    /// If true, rotate all scenes to match the target semantics.
    ///
    /// This only affects the transforms of the root node entities of each
    /// scene.
    pub rotate_scenes: bool,

    /// If true, rotate the local transforms of nodes to match the target
    /// semantics. This affects the transforms of nodes and their immediate
    /// child entities.
    ///
    /// Nodes that are cameras or lights are *not* converted. This is because
    /// both glTF and Bevy require camera and light nodes to be -Z forward.
    pub rotate_nodes: bool,

    /// If true, rotate all meshes to match the target semantics.
    ///
    /// This can affect:
    /// - The vertices of [`Mesh`] assets, including morph targets.
    /// - The transform and [`Aabb`](bevy_camera::primitives::Aabb) components
    ///   of entities that instance meshes through a [`Mesh3d`](bevy_mesh::Mesh3d)
    ///   component.
    /// - The matrices in [`SkinnedMeshInverseBindposes`](bevy_mesh::skinning::SkinnedMeshInverseBindposes)
    ///   assets.
    pub rotate_meshes: bool,

    /// The semantics conversion to use if any of `rotate_scenes`, `rotate_nodes`
    /// or `rotate_meshes` are enabled.
    ///
    /// Defaults to converting from [glTF semantics](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#coordinate-system-and-units)
    /// (+Z forward, +Y up) to Bevy semantics (-Z forward, +Y up).
    pub semantics: GltfConvertSemantics,
}

impl GltfConvertCoordinates {
    /// Convert scenes, nodes and meshes from glTF to Bevy semantics.
    pub const ALL: Self = Self {
        rotate_scenes: true,
        rotate_nodes: true,
        rotate_meshes: true,
        semantics: GltfConvertSemantics::All(SemanticsConversion::GLTF_TO_BEVY),
    };
}

/// The semantics conversion that will be used by scenes, nodes and meshes.
///
/// These options are only used if the corresponding [`rotate_scenes`](GltfConvertCoordinates::rotate_scenes),
/// [`rotate_nodes`](GltfConvertCoordinates::rotate_nodes), and [`rotate_meshes`](GltfConvertCoordinates::rotate_meshes)
/// options are enabled.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum GltfConvertSemantics {
    /// Use one semantics conversion for scenes, nodes and meshes.
    All(SemanticsConversion),
    /// Use different semantics conversions for scenes, nodes and meshes.
    Separate {
        /// The semantics conversion for scenes.
        scenes: SemanticsConversion,
        /// The semantics conversion for nodes.
        nodes: SemanticsConversion,
        /// The semantics conversion for scenes
        meshes: SemanticsConversion,
    },
}

impl GltfConvertSemantics {
    fn scenes(&self) -> SemanticsConversion {
        match self {
            Self::All(all) => *all,
            Self::Separate { scenes, .. } => *scenes,
        }
    }

    fn nodes(&self) -> SemanticsConversion {
        match self {
            Self::All(all) => *all,
            Self::Separate { nodes, .. } => *nodes,
        }
    }

    fn meshes(&self) -> SemanticsConversion {
        match self {
            Self::All(all) => *all,
            Self::Separate { meshes, .. } => *meshes,
        }
    }
}

impl Default for GltfConvertSemantics {
    fn default() -> Self {
        GltfConvertSemantics::All(SemanticsConversion::GLTF_TO_BEVY)
    }
}

/// A [`GltfConvertCoordinates`] that has been resolved to converters.
#[derive(Debug)]
pub(crate) struct ResolvedConvertCoordinates {
    scene_converter: RotationConverter,
    node_converter: RotationConverter,
    mesh_converter: RotationConverter,
}

impl ResolvedConvertCoordinates {
    pub(crate) fn node_rotation_converter(&self, node: &Node) -> RotationConverter {
        if node.camera().is_some() || node.light().is_some() {
            // Cameras and lights are not converted. glTF requires cameras and
            // lights to be -Z forward, which means they already follow Bevy
            // semantics.
            RotationConverter::IDENTITY
        } else {
            self.node_converter
        }
    }

    pub(crate) fn node_hierarchy_converter(
        &self,
        node: &Node,
        parents: &[Option<Node>],
    ) -> HierarchyConverter {
        let parent_node = parents.get(node.index()).cloned().flatten();

        let parent_converter = if let Some(parent_node) = parent_node {
            self.node_rotation_converter(&parent_node)
        } else {
            self.scene_converter
        };

        let local_converter = self.node_rotation_converter(node);

        HierarchyConverter::from_local_and_parent(local_converter, parent_converter)
    }

    pub(crate) fn mesh_entity_rotation_converter(&self) -> RotationConverter {
        self.mesh_converter
    }

    /// Returns the hierarchy converter for mesh entities, which are assumed to
    /// be children of node entities.
    pub(crate) fn mesh_entity_hierarchy_converter(&self, parent_node: &Node) -> HierarchyConverter {
        HierarchyConverter::from_local_and_parent(
            self.mesh_converter,
            self.node_rotation_converter(parent_node),
        )
    }

    // Returns the hierarchy converter for mesh vertices.
    pub(crate) fn mesh_vertex_hierarchy_converter(&self) -> HierarchyConverter {
        // Mesh vertices are considered children of the mesh entities. The
        // semantics of mesh vertices are not converted, but their translation
        // is affected by the mesh entity's conversion.
        HierarchyConverter::from_local_and_parent(RotationConverter::IDENTITY, self.mesh_converter)
    }
}

impl TryFrom<GltfConvertCoordinates> for ResolvedConvertCoordinates {
    type Error = SemanticsError;

    fn try_from(value: GltfConvertCoordinates) -> Result<Self, Self::Error> {
        Ok(Self {
            scene_converter: if value.rotate_scenes {
                value.semantics.scenes().try_into()?
            } else {
                RotationConverter::IDENTITY
            },
            node_converter: if value.rotate_nodes {
                value.semantics.nodes().try_into()?
            } else {
                RotationConverter::IDENTITY
            },
            mesh_converter: if value.rotate_meshes {
                value.semantics.meshes().try_into()?
            } else {
                RotationConverter::IDENTITY
            },
        })
    }
}

#[derive(Error, Debug)]
pub(crate) enum CoordinateConversionAttributeError {
    #[error("Cannot apply coordinate conversion to attribute {0} - unsupported format {1:?}")]
    UnsupportedFormat(&'static str, VertexFormat),
}

/// Apply the given converter to the attribute.
pub(crate) fn convert_attribute_coordinates(
    attribute: MeshVertexAttribute,
    values: VertexAttributeValues,
    converter: HierarchyConverter,
) -> Result<VertexAttributeValues, CoordinateConversionAttributeError> {
    if converter == HierarchyConverter::IDENTITY {
        return Ok(values);
    }

    match attribute {
        Mesh::ATTRIBUTE_POSITION | Mesh::ATTRIBUTE_NORMAL | Mesh::ATTRIBUTE_TANGENT => match values
        {
            VertexAttributeValues::Float32x3(mut values) => {
                for value in &mut values {
                    *value = converter
                        .convert_translation(Vec3::from_array(*value))
                        .to_array();
                }
                Ok(VertexAttributeValues::Float32x3(values))
            }

            VertexAttributeValues::Float32x4(mut values) => {
                for value in &mut values {
                    *value = converter
                        .convert_translation(Vec4::from_array(*value).truncate())
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

/// An axis in a 3D cartesian coordinate system.
#[expect(missing_docs, reason = "The variants are self-explanatory")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Axis {
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
        Dir3::from(value).into()
    }
}

impl From<Axis> for Dir3 {
    fn from(value: Axis) -> Self {
        match value {
            Axis::X => Dir3::X,
            Axis::Y => Dir3::Y,
            Axis::Z => Dir3::Z,
        }
    }
}

/// A positive or negative sign. Used by [`SignedAxis`].
#[expect(missing_docs, reason = "The variants are self-explanatory")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sign {
    Positive,
    Negative,
}

impl Sign {
    /// Returns true if the sign is positive.
    pub fn is_positive(&self) -> bool {
        *self == Self::Positive
    }

    /// Returns true if the sign is negative.
    pub fn is_negative(&self) -> bool {
        *self == Self::Negative
    }

    /// Returns `1.0` if positive or `-1.0` if negative.
    pub fn multiplier(&self) -> f32 {
        match self {
            Self::Positive => 1.0,
            Self::Negative => -1.0,
        }
    }
}

/// A signed 3D axis, for example "+X" or "-Z".
#[expect(missing_docs, reason = "The members are self-explanatory")]
#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedAxis {
    pub sign: Sign,
    pub axis: Axis,
}

impl SignedAxis {
    /// +X
    pub const X: Self = Self {
        sign: Sign::Positive,
        axis: Axis::X,
    };

    /// +Y
    pub const Y: Self = Self {
        sign: Sign::Positive,
        axis: Axis::Y,
    };

    /// +Z
    pub const Z: Self = Self {
        sign: Sign::Positive,
        axis: Axis::Z,
    };

    /// -X
    pub const NEG_X: Self = Self {
        sign: Sign::Negative,
        axis: Axis::X,
    };

    /// -Y
    pub const NEG_Y: Self = Self {
        sign: Sign::Negative,
        axis: Axis::Y,
    };

    /// -Z
    pub const NEG_Z: Self = Self {
        sign: Sign::Negative,
        axis: Axis::Z,
    };
}

impl Debug for SignedAxis {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self.sign {
            Sign::Positive => "+",
            Sign::Negative => "-",
        })?;
        self.axis.fmt(f)
    }
}

impl From<SignedAxis> for Vec3 {
    fn from(value: SignedAxis) -> Self {
        Dir3::from(value).into()
    }
}

impl From<SignedAxis> for Dir3 {
    fn from(value: SignedAxis) -> Self {
        match value.sign {
            Sign::Positive => Dir3::from(value.axis),
            Sign::Negative => -Dir3::from(value.axis),
        }
    }
}

/// Defines forward/up/right semantics for a cartesian coordinate system, where
/// the forward and up axes are explicit but the right is implicit.
///
/// Not guarantee to be valid. See [`ValidSemantics`] for explicit and validated
/// semantics.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Semantics {
    /// The forward axis.
    pub forward: SignedAxis,

    /// The up axis.
    pub up: SignedAxis,
}

impl Semantics {
    /// Returns semantics with the given forward and up.
    pub fn from_forward_up(forward: SignedAxis, up: SignedAxis) -> Self {
        Semantics { forward, up }
    }

    /// Bevy semantics: -Z forward, +Y up.
    pub const BEVY: Self = Self {
        forward: SignedAxis::NEG_Z,
        up: SignedAxis::Y,
    };

    /// [Standard glTF semantics](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#coordinate-system-and-units):
    /// +Z forward, +Y up. This excludes glTF cameras and lights, which are -Z forward, +Y up.
    pub const GLTF: Self = Self {
        forward: SignedAxis::Z,
        up: SignedAxis::Y,
    };
}

/// Defines forward/up/right semantics for a cartesian coordinate system.
///
/// Guaranteed to be valid - each semantic is a different axis, and the right
/// is the cross product of forward and up.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ValidSemantics {
    forward: SignedAxis,
    up: SignedAxis,
    right: SignedAxis,
}

impl ValidSemantics {
    /// Returns the forward axis.
    pub fn forward(&self) -> SignedAxis {
        self.forward
    }

    /// Returns the up axis.
    pub fn up(&self) -> SignedAxis {
        self.up
    }

    /// Returns the right axis.
    pub fn right(&self) -> SignedAxis {
        self.right
    }

    /// Bevy semantics: -Z forward, +Y up.
    pub const BEVY: Self = Self {
        forward: SignedAxis::NEG_Z,
        up: SignedAxis::Y,
        right: SignedAxis::X,
    };

    /// [Standard glTF semantics](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#coordinate-system-and-units):
    /// +Z forward, +Y up. This excludes glTF cameras and lights, which are -Z forward, +Y up.
    pub const GLTF: Self = Self {
        forward: SignedAxis::Z,
        up: SignedAxis::Y,
        right: SignedAxis::NEG_X,
    };
}

impl TryFrom<Semantics> for ValidSemantics {
    type Error = SemanticsError;

    fn try_from(value: Semantics) -> Result<Self, Self::Error> {
        let forward = value.forward;
        let up = value.up;

        if forward.axis == up.axis {
            return Err(SemanticsError::ForwardAndUpAreSameAxis(forward.axis));
        }

        // Equivalent to `right = forward.cross(up)`.

        let winding = (forward.axis.index() + 1).rem_euclid(3) == up.axis.index();

        let right = SignedAxis {
            sign: match winding ^ forward.sign.is_positive() ^ up.sign.is_positive() {
                true => Sign::Positive,
                false => Sign::Negative,
            },
            // SAFETY: We know `forward.axis != up.axis`, therefore `forward.axis.index() + up.axis.index()`
            // must be 1, 2 or 3.
            axis: Axis::from_index_unchecked(3 - (forward.axis.index() + up.axis.index())),
        };

        Ok(Self { forward, up, right })
    }
}

/// Errors from converting [`Semantics`] to [`ValidSemantics`].
#[derive(Error, Debug, PartialEq, Eq)]
pub enum SemanticsError {
    /// Forward and up are the same axis.
    #[error("Forward and up are the same axis: {0:?}.")]
    ForwardAndUpAreSameAxis(Axis),
}

/// Describes a conversion from source to target [`Semantics`], for example
/// [`GLTF_TO_BEVY`](SemanticsConversion::GLTF_TO_BEVY).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SemanticsConversion {
    /// The semantics to convert from.
    pub source: Semantics,

    /// The semantics to convert to.
    pub target: Semantics,
}

impl SemanticsConversion {
    /// Converts from glTF semantics (+Z forward, +Y up) to Bevy semantics
    /// (-Z forward, +Y up).
    pub const GLTF_TO_BEVY: Self = Self {
        source: Semantics::GLTF,
        target: Semantics::BEVY,
    };
}

/// Converts between semantics by swapping and/or flipping the X/Y/Z components
/// of a vector.
///
/// This is effectively a rotation, but is usually faster and more accurate than
/// rotating by a matrix or quaternion. It can also convert scales.
///
/// The behavior of the conversion can be unintuitive. If the source semantics
/// are "+X forward" and the target semantics are "+Z forward", then converting
/// `Vec3::X` will *not* return `Vec3::Z`. Instead the reverse happens -
/// converting `Vec3::Z` will return `Vec3::X`.
///
/// One way to explain this is that we're not converting something *within* a
/// coordinate system - we're converting the coordinate system *itself*. So when
/// converting a node in a scene hierarchy from "+X forward" to "+Z forward",
/// we're rotating it so that the new +Z forward axis is the same direction as
/// the old +X forward axis.
///
/// ```text
///                 Before            After
///     ^
///     |            x                   z
///     |            |                   |
///     |            O--z             x--O
///  Forward
/// ```

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct RemappingConverter {
    /// Maps from source axis index to target axis.
    source_to_target: [Axis; 3],

    /// Sign flip, applied by multiplying after `source_to_target`.
    flip: Vec3,
}

impl RemappingConverter {
    pub(crate) fn from_source_target(
        source: ValidSemantics,
        target: ValidSemantics,
    ) -> RemappingConverter {
        let (sf, su, sr) = (source.forward(), source.up(), source.right());
        let (tf, tu, tr) = (target.forward(), target.up(), target.right());

        let mut source_to_target = [Axis::X; 3];

        source_to_target[sf.axis.index()] = tf.axis;
        source_to_target[su.axis.index()] = tu.axis;
        source_to_target[sr.axis.index()] = tr.axis;

        let mut flip = Vec3::ZERO;

        flip[sf.axis.index()] = tf.sign.multiplier() * sf.sign.multiplier();
        flip[su.axis.index()] = tu.sign.multiplier() * su.sign.multiplier();
        flip[sr.axis.index()] = tr.sign.multiplier() * sr.sign.multiplier();

        Self {
            source_to_target,
            flip,
        }
    }

    pub(crate) const IDENTITY: RemappingConverter = RemappingConverter {
        source_to_target: [Axis::X, Axis::Y, Axis::Z],
        flip: Vec3::ONE,
    };

    pub(crate) fn convert_translation(&self, value: Vec3) -> Vec3 {
        Vec3::new(
            value[self.source_to_target[0].index()],
            value[self.source_to_target[1].index()],
            value[self.source_to_target[2].index()],
        ) * self.flip
    }

    pub(crate) fn inverse_convert_translation(&self, value: Vec3) -> Vec3 {
        let mut result = Vec3::ZERO;

        result[self.source_to_target[0].index()] = value[0] * self.flip[0];
        result[self.source_to_target[1].index()] = value[1] * self.flip[1];
        result[self.source_to_target[2].index()] = value[2] * self.flip[2];

        result
    }

    #[expect(unused, reason = "Currently unused, but kept for future use.")]
    pub(crate) fn convert_scale(&self, value: Vec3) -> Vec3 {
        Vec3::new(
            value[self.source_to_target[0].index()],
            value[self.source_to_target[1].index()],
            value[self.source_to_target[2].index()],
        )
    }

    pub(crate) fn inverse_convert_scale(&self, value: Vec3) -> Vec3 {
        let mut result = Vec3::ZERO;

        result[self.source_to_target[0].index()] = value[0];
        result[self.source_to_target[1].index()] = value[1];
        result[self.source_to_target[2].index()] = value[2];

        result
    }
}

/// A convenient bundle of equivalent rotation conversions.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct RotationConverter {
    rotation: Quat,
    matrix: Mat4,
    remapping: RemappingConverter,
}

impl TryFrom<SemanticsConversion> for RotationConverter {
    type Error = SemanticsError;

    fn try_from(value: SemanticsConversion) -> Result<Self, Self::Error> {
        Ok(Self::from_source_target(
            value.source.try_into()?,
            value.target.try_into()?,
        ))
    }
}

impl RotationConverter {
    const IDENTITY: RotationConverter = RotationConverter {
        rotation: Quat::IDENTITY,
        matrix: Mat4::IDENTITY,
        remapping: RemappingConverter::IDENTITY,
    };

    pub(crate) fn from_source_target(
        source: ValidSemantics,
        target: ValidSemantics,
    ) -> RotationConverter {
        let remapping = RemappingConverter::from_source_target(source, target);

        let matrix = Mat4::from_cols(
            remapping
                .convert_translation(vec3(1.0, 0.0, 0.0))
                .extend(0.0),
            remapping
                .convert_translation(vec3(0.0, 1.0, 0.0))
                .extend(0.0),
            remapping
                .convert_translation(vec3(0.0, 0.0, 1.0))
                .extend(0.0),
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        );

        // TODO: Mat4 to Quat gives the right result, but another method could
        // be more efficient?
        let rotation = Quat::from_mat4(&matrix);

        Self {
            rotation,
            matrix,
            remapping,
        }
    }

    pub(crate) fn rotation(&self) -> Quat {
        self.rotation
    }

    pub(crate) fn matrix(&self) -> Mat4 {
        self.matrix
    }

    pub(crate) fn remapping(&self) -> RemappingConverter {
        self.remapping
    }
}

/// Helper for converting the semantics of transforms in a scene hierarchy,
/// where each transform may have different semantics.
///
/// To explain how this works, let's start with a scene containing one mesh. The
/// diagram shows a top-down view of the scene - the axes in the middle of the
/// mesh represent mesh-space, and the axes in the bottom left represent
/// scene-space.
///
/// ```text
///        +-----------+
///        |           +-+
///        |      x      |
///        |      |      |
///        |      M--z   |
///        |             |
/// x      +-------------+
/// |
/// S--z
/// ```
///
/// We want to convert the mesh's semantics from +X forward to +Z forward, but
/// the mesh's vertices should stay at the same position in scene-space. This
/// can be done by rotating the mesh 90 degrees counter-clockwise, then
/// rotating its vertices 90 degrees clockwise.
///
/// ```text
///          Before                   Rotate Mesh        Counter-Rotate Vertices
///
///        +-----------+              +-----------+          +-----------+
///        |      x    +-+          +-+    z      |          |      z    +-+
///        |      |      |          |      |      |          |      |      |
///        |      M--z   |          |   x--M      |          |   x--M      |
///        |             |          |             |          |             |
///        |             |          |             |          |             |
/// x      +-------------+          +-------------+          +-------------+
/// |
/// S--z
/// ```
///
/// Conceptually, the scene has the following hierarchy:
///
/// - Scene.
///   - Mesh.
///     - Mesh vertices.
///
/// Only the mesh is having its semantics converted - the semantics of the scene
/// and vertices stay the same.
///
/// The rules for converting the semantics of a transform in a hierarchy are:
///
/// - Apply the transform's rotation conversion to the local-space of the
///   transform.
/// - Apply the parent's *inverse* rotation conversion to the parent-space of
///   the transform.
///
/// The inverse parent rotation is needed because we want to maintain the
/// scene-space position of the child. So when the parent is rotated we need the
/// child to compensate by applying the opposite rotation.
///
/// Applying these rules to the mesh's vertices, we get:
///
/// - Apply the vertices' conversion to the vertices - the vertices have no
///   conversion so nothing is done.
/// - Apply the mesh's inverse conversion to the vertices.
///
/// And for the mesh:
///
/// - Apply the mesh's conversion to the mesh.
/// - Apply the scene's inverse conversion to the mesh - the scene has no
///   conversion so   nothing is done.
///
/// If we take both cases and remove the rules that don't do anything, we end up
/// with:
///
/// - Apply the mesh's conversion to the mesh.
/// - Apply the mesh's inverse conversion to the vertices.
///
/// Note that even though the semantics of the vertices are not being converted,
/// their translation is still affected by the mesh's conversion.
///
/// Let's apply the same rules to scene nodes. Say we have two nodes A and B,
/// where A is the parent of B, and we want to convert the semantics of both A
/// and B from +X forward to +Z forward. The conversion should look like this:
///
/// ```text
///          Before                After
///
///                x                       z
///                |                       |
///                B--z                 x--B
///              .`                      .`
///        X   .`                  z   .`
///        | .`                    | .`
///        A--z                 x--A
/// ```
///
/// Unlike the mesh example, both the parent and the child want the same
/// conversion. So the rules for A are:
///
/// 1. Apply the scene's inverse conversion to A - the scene has no conversion.
/// 2. Apply A's conversion to A.
///
/// And B:
///
/// 1. Apply A's inverse conversion to B.
/// 2. Apply B's conversion to B.
///
/// Applying these step by step, we get:
///
/// ```text
///                Before                     Apply A's conversion to A
///
///                         x                      z
///                         |                      |
///                         B--z                x--B
///                       .`                        `.
///                 x   .`                            `.  z
///                 | .`                                `.|
///                 A--z                               x--A
///
///
///    Apply A's inverse conversion to B      Apply B's conversion to B
///
///                         x                                     z
///                         |                                     |
///                         B--z                               x--B
///                       .`                                    .`
///                 x   .`                                z   .`
///                 | .`                                  | .`
///                 A--z                               x--A
/// ```
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct HierarchyConverter {
    local: RotationConverter,
    parent: RotationConverter,
}

impl HierarchyConverter {
    const IDENTITY: Self = Self {
        local: RotationConverter::IDENTITY,
        parent: RotationConverter::IDENTITY,
    };

    pub(crate) fn from_local_and_parent(
        local: RotationConverter,
        parent: RotationConverter,
    ) -> Self {
        Self { local, parent }
    }

    pub(crate) fn convert_translation(&self, t: Vec3) -> Vec3 {
        self.parent.remapping().inverse_convert_translation(t)
    }

    pub(crate) fn convert_rotation(&self, r: Quat) -> Quat {
        self.parent.rotation().inverse() * r * self.local.rotation()
    }

    pub(crate) fn convert_scale(&self, s: Vec3) -> Vec3 {
        self.local.remapping().inverse_convert_scale(s)
    }

    pub(crate) fn convert_transform(&self, t: Transform) -> Transform {
        Transform::from_translation(self.convert_translation(t.translation))
            .with_rotation(self.convert_rotation(t.rotation))
            .with_scale(self.convert_scale(t.scale))
    }

    pub(crate) fn convert_aabb(&self, aabb: Aabb3d) -> Aabb3d {
        let min = self
            .parent
            .remapping()
            .inverse_convert_translation(aabb.min.into());
        let max = self
            .parent
            .remapping()
            .inverse_convert_translation(aabb.max.into());

        Aabb3d::from_min_max(min.min(max), min.max(max))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_transform::components::GlobalTransform;
    use rand::{distr::Distribution, rngs::StdRng, seq::IndexedRandom, Rng, RngExt, SeedableRng};

    const SIGNED_AXES: [SignedAxis; 6] = [
        SignedAxis::X,
        SignedAxis::Y,
        SignedAxis::Z,
        SignedAxis::NEG_X,
        SignedAxis::NEG_Y,
        SignedAxis::NEG_Z,
    ];

    fn semantics_permutations() -> impl Iterator<Item = ValidSemantics> {
        SIGNED_AXES.iter().flat_map(|forward| {
            SIGNED_AXES
                .iter()
                .filter(|up| forward.axis != up.axis)
                .map(|up| {
                    ValidSemantics::try_from(Semantics::from_forward_up(*forward, *up)).unwrap()
                })
        })
    }

    // Test that all combinations of semantics are valid.
    #[test]
    fn semantics() {
        for semantics in semantics_permutations() {
            let forward = Vec3::from(semantics.forward());
            let up = Vec3::from(semantics.up());
            let right = Vec3::from(semantics.right());

            assert_eq!(right, Vec3::cross(forward, up), "{semantics:?}");
        }

        assert_eq!(
            Err(SemanticsError::ForwardAndUpAreSameAxis(Axis::Z)),
            ValidSemantics::try_from(Semantics {
                forward: SignedAxis::Z,
                up: SignedAxis::NEG_Z
            })
        );
    }

    // Test that our named semantics are valid. Also test that the Bevy
    // semantics match `Transform`.
    #[test]
    fn named_semantics() {
        assert_eq!(
            ValidSemantics::GLTF,
            ValidSemantics::try_from(Semantics::GLTF).unwrap()
        );

        assert_eq!(
            ValidSemantics::BEVY,
            ValidSemantics::try_from(Semantics::BEVY).unwrap()
        );

        assert_eq!(
            Dir3::from(ValidSemantics::BEVY.forward()),
            Transform::IDENTITY.forward()
        );

        assert_eq!(
            Dir3::from(ValidSemantics::BEVY.up()),
            Transform::IDENTITY.up()
        );

        assert_eq!(
            Dir3::from(ValidSemantics::BEVY.right()),
            Transform::IDENTITY.right()
        );
    }

    // For coverage, create a test direction that has a mix of forward, up, and
    // right.
    fn test_direction(semantics: ValidSemantics) -> Vec3 {
        (Vec3::from(semantics.forward()) * 3.0)
            + (Vec3::from(semantics.up()) * 2.0)
            + Vec3::from(semantics.right())
    }

    // Test that all the converters in `RotationConverter` are correct and
    // equivalent.
    #[test]
    fn rotation_converter() {
        for source_semantics in semantics_permutations() {
            for target_semantics in semantics_permutations() {
                let converter =
                    RotationConverter::from_source_target(source_semantics, target_semantics);

                let source_direction = test_direction(source_semantics);
                let target_direction = test_direction(target_semantics);

                let remapping = converter.remapping().convert_translation(target_direction);
                let rotation = converter.rotation() * target_direction;
                let matrix = converter.matrix().transform_vector3(target_direction);

                assert!(source_direction.abs_diff_eq(remapping, 1e-6));
                assert!(source_direction.abs_diff_eq(rotation, 1e-6));
                assert!(source_direction.abs_diff_eq(matrix, 1e-6));
            }
        }
    }

    // A distribution of random transforms within a narrow range. This keeps the
    // error bounds small enough that we can test for equality with a simple
    // `abs_diff_eq` and fixed epsilon.
    struct RandomSmallTransforms;

    impl Distribution<Transform> for RandomSmallTransforms {
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Transform {
            let signs = [1.0, -1.0];

            Transform::from_xyz(
                rng.random_range(-2.0..2.0),
                rng.random_range(-2.0..2.0),
                rng.random_range(-2.0..2.0),
            )
            .with_rotation(rng.random())
            .with_scale(vec3(
                rng.random_range(0.5..1.5) * signs.choose(rng).unwrap(),
                rng.random_range(0.5..1.5) * signs.choose(rng).unwrap(),
                rng.random_range(0.5..1.5) * signs.choose(rng).unwrap(),
            ))
        }
    }

    struct RandomSemantics(Vec<ValidSemantics>);

    impl Distribution<ValidSemantics> for RandomSemantics {
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ValidSemantics {
            *self.0.choose(rng).unwrap()
        }
    }

    struct RandomConverters(RandomSemantics);

    impl Distribution<RotationConverter> for RandomConverters {
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> RotationConverter {
            RotationConverter::from_source_target(self.0.sample(rng), self.0.sample(rng))
        }
    }

    // Given a hierarchy of node transforms in root to leaf order, and a transform
    // in the leaf node's space, convert the transform to scene-space.
    fn scenespace_transform(input: Transform, hierarchy: &[Transform]) -> GlobalTransform {
        hierarchy
            .iter()
            .rev()
            .fold(input.into(), |a, &t| GlobalTransform::from(t) * a)
    }

    // Generate random hierarchies and test that that they produce the same
    // scene-space transform after applying random conversions.
    #[test]
    fn hierarchy() {
        let mut rng = StdRng::seed_from_u64(1234);

        let random_converters =
            RandomConverters(RandomSemantics(semantics_permutations().collect()));

        for _ in 0..100 {
            let node_converters = [
                random_converters.sample(&mut rng),
                random_converters.sample(&mut rng),
            ];

            let hierarchy_converters = [
                HierarchyConverter::from_local_and_parent(
                    node_converters[0],
                    RotationConverter::IDENTITY,
                ),
                HierarchyConverter::from_local_and_parent(node_converters[1], node_converters[0]),
                // The leaf node uses the identity conversion. This means that
                // whatever conversions are applied to other nodes in the
                // hierarchy, a transform in the leaf node's space should always
                // result in the same transform in scene-space.
                HierarchyConverter::from_local_and_parent(
                    RotationConverter::IDENTITY,
                    node_converters[1],
                ),
            ];

            let original_hierarchy = RandomSmallTransforms
                .sample_iter(&mut rng)
                .take(hierarchy_converters.len())
                .collect::<Vec<_>>();

            let converted_hierarchy = original_hierarchy
                .iter()
                .zip(hierarchy_converters.iter())
                .map(|(&transform, converter)| converter.convert_transform(transform))
                .collect::<Vec<_>>();

            let local = RandomSmallTransforms.sample(&mut rng);
            let original = scenespace_transform(local, &original_hierarchy);
            let converted = scenespace_transform(local, &converted_hierarchy);

            assert!(original.affine().abs_diff_eq(converted.affine(), 1e-4));
        }
    }
}
