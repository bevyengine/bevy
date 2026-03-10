//! Utilities for converting glTF's [standard coordinate system](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#coordinate-system-and-units)
//! to Bevy's.
use bevy_mesh::{Mesh, MeshVertexAttribute, VertexAttributeValues, VertexFormat};
use core::fmt::Debug;
use gltf::Node;
use serde::{Deserialize, Serialize};

use bevy_math::{bounding::Aabb3d, vec3, Mat4, Quat, Vec3, Vec4};
use bevy_transform::components::Transform;
use thiserror::Error;

/// XXX TODO
///
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
    /// If true, rotate all scenes to match the target semantics.
    ///
    /// This only affects the transforms of the root nodes of each scene.
    pub rotate_scenes: bool,

    /// If true, rotate the local transforms of all nodes to match target semantics.
    ///
    /// This affects the transforms of nodes and their immediate child entities.
    ///
    /// Nodes that are cameras or lights are *not* changed, except to counter
    /// any conversion of their parent node. This is because both glTF and Bevy
    /// require camera and light nodes to be -Z forward.
    pub rotate_nodes: bool,

    /// If true, rotate all meshes to match the target semantics.
    ///
    /// This can affect:
    /// - The vertices of [`Mesh`](bevy_mesh::Mesh) assets, including morph targets.
    /// - The transforms and [`Aabb`](bevy_camera::primitives::Aabb) components
    ///   of entities that instance meshes through a [`Mesh3d`](bevy_mesh::Mesh3d)
    ///   component.
    /// - The matrices in [`SkinnedMeshInverseBindposes`](bevy_mesh::skinning::SkinnedMeshInverseBindposes)
    ///   assets.
    pub rotate_meshes: bool,

    /// The semantics conversion to use if any of `rotate_scenes`, `rotate_nodes`
    /// or `rotate_meshes` are enabled. Defaults to ['GLTF_TO_BEVY](SemanticsConversion::GLTF_TO_BEVY].
    pub semantics: GltfConvertSemantics,
}

/// The semantics conversion options that will be used by scenes, nodes and
/// meshes.
///
/// These options are only used if the corresponding [`rotate_scenes`](GltfConvertCoordinates::rotate_scenes),
/// [`rotate_nodes`](GltfConvertCoordinates::rotate_nodes), and [`rotate_meshes`](GltfConvertCoordinates::rotate_meshes)
/// options are enabled.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum GltfConvertSemantics {
    /// Use one semantics conversion for scenes, nodes and meshes. They
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

/// A `GltfConvertCoordinates` that's been resolved to converters.
#[derive(Debug)]
pub(crate) struct ResolvedConvertCoordinates {
    scene: Converter,
    node: Converter,
    mesh: Converter,
}

impl ResolvedConvertCoordinates {
    pub(crate) fn resolve(input: GltfConvertCoordinates) -> Result<Self, SemanticsError> {
        Ok(Self {
            scene: if input.rotate_scenes {
                input.semantics.scenes().try_into()?
            } else {
                Converter::IDENTITY
            },
            node: if input.rotate_nodes {
                input.semantics.nodes().try_into()?
            } else {
                Converter::IDENTITY
            },
            mesh: if input.rotate_meshes {
                input.semantics.meshes().try_into()?
            } else {
                Converter::IDENTITY
            },
        })
    }

    pub(crate) fn scene(&self) -> Converter {
        self.scene
    }

    pub(crate) fn node(&self, node: &Node) -> Converter {
        if node.camera().is_none() && node.light().is_none() {
            self.node
        } else {
            Converter::IDENTITY
        }
    }

    pub(crate) fn mesh(&self) -> Converter {
        self.mesh
    }

    pub(crate) fn node_hierarchy_conversion(
        &self,
        node: &Node,
        parent_node: Option<&Node>,
    ) -> HierarchyConverter {
        let parent_converter = if let Some(parent_node) = parent_node {
            self.node(parent_node)
        } else {
            self.scene
        };

        let local_converter = self.node(node);

        HierarchyConverter::from_local_and_parent(local_converter, parent_converter)
    }

    pub(crate) fn mesh_hierarchy_conversion(&self, node: &Node) -> HierarchyConverter {
        HierarchyConverter::from_local_and_parent(self.mesh, self.node(node))
    }

    pub(crate) fn mesh_vertex_converter(&self) -> RemappingConverter {
        self.mesh.remapping()
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
    converter: RemappingConverter,
) -> Result<VertexAttributeValues, CoordinateConversionAttributeError> {
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

/// A axis in a 3d cartesian coordinate system.
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
        match value {
            Axis::X => vec3(1.0, 0.0, 0.0),
            Axis::Y => vec3(0.0, 1.0, 0.0),
            Axis::Z => vec3(0.0, 0.0, 1.0),
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

/// A signed 3D axis, e.g. "+X", "-Z".
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
        match value.sign {
            Sign::Positive => Vec3::from(value.axis),
            Sign::Negative => -Vec3::from(value.axis),
        }
    }
}

/// Defines forward/up/right semantics for a cartesian coordinate system.
///
/// The right axis is left implicit - it will be the cross product of the
/// forward and up axes.
///
/// Not guarantee to be valid. See `Semantics` for explicit and validated
/// semantics.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Semantics {
    /// The forward axis.
    pub forward: SignedAxis,

    /// The up axis.
    pub up: SignedAxis,
}

impl Semantics {
    /// Standard Bevy semantics: -Z forward, +Y up.
    pub const BEVY: Self = Self {
        forward: SignedAxis::NEG_Z,
        up: SignedAxis::Y,
    };

    /// [Standard glTF semantics](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#coordinate-system-and-units):
    /// +Z forward, +Y up (excluding cameras and lights, which are -Z forward, +Y up).
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
    /// Create semantics from forward and up axes. The right axis is implicit.
    pub fn from_forward_up(forward: SignedAxis, up: SignedAxis) -> Result<Self, SemanticsError> {
        TryFrom::try_from(Semantics { forward, up })
    }

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

    /// Standard Bevy semantics: -Z forward, +Y up.
    pub const BEVY: Self = Self {
        forward: SignedAxis::NEG_Z,
        up: SignedAxis::Y,
        right: SignedAxis::X,
    };

    /// [Standard glTF semantics](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#coordinate-system-and-units):
    /// +Z forward, +Y up (excluding cameras and lights, which are -Z forward, +Y up).
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
            return Err(SemanticsError::ForwardAndUpAreSameAxis(value.forward.axis));
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

/// Describes a conversion from source to target `Semantics`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SemanticsConversion {
    /// The semantics to convert from.
    pub source: Semantics,

    /// The semantics to convert to.
    pub target: Semantics,
}

impl SemanticsConversion {
    /// Converts from glTF to Bevy semantics.
    const GLTF_TO_BEVY: Self = Self {
        source: Semantics::GLTF,
        target: Semantics::BEVY,
    };
}

/// Converts between semantics by swapping and/or flipping components. This
/// is usually faster and more accurate than using a matrix or quaternion.
#[derive(Copy, Clone, Debug)]
pub(crate) struct RemappingConverter {
    /// Maps from target to source axis.
    axis: [Axis; 3],

    /// Sign flip, applied by multiplying after `axis` remapping,
    flip: Vec3,
}

impl RemappingConverter {
    pub(crate) fn from_source_target(
        source: ValidSemantics,
        target: ValidSemantics,
    ) -> RemappingConverter {
        let (sf, su, sr) = (source.forward(), source.up(), source.right());
        let (tf, tu, tr) = (target.forward(), target.up(), target.right());

        let mut axis = [Axis::X; 3];

        axis[tf.axis.index()] = sf.axis;
        axis[tu.axis.index()] = su.axis;
        axis[tr.axis.index()] = sr.axis;

        let mut flip = Vec3::ZERO;

        flip[tf.axis.index()] = tf.sign.multiplier() * sf.sign.multiplier();
        flip[tu.axis.index()] = tu.sign.multiplier() * su.sign.multiplier();
        flip[tr.axis.index()] = tr.sign.multiplier() * sr.sign.multiplier();

        Self { axis, flip }
    }

    pub(crate) const IDENTITY: RemappingConverter = RemappingConverter {
        axis: [Axis::X, Axis::Y, Axis::Z],
        flip: Vec3::ONE,
    };

    pub(crate) fn convert_translation(&self, source: Vec3) -> Vec3 {
        Vec3::new(
            source[self.axis[0].index()],
            source[self.axis[1].index()],
            source[self.axis[2].index()],
        ) * self.flip
    }

    pub(crate) fn convert_scale(&self, source: Vec3) -> Vec3 {
        Vec3::new(
            source[self.axis[0].index()],
            source[self.axis[1].index()],
            source[self.axis[2].index()],
        )
    }

    pub(crate) fn convert_aabb(&self, source: Aabb3d) -> Aabb3d {
        let min = self.convert_translation(source.min.into());
        let max = self.convert_translation(source.max.into());

        Aabb3d::from_min_max(min.min(max), min.max(max))
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct Converter {
    rotation: Quat,
    matrix: Mat4,
    remapping: RemappingConverter,
}

impl TryFrom<SemanticsConversion> for Converter {
    type Error = SemanticsError;

    fn try_from(value: SemanticsConversion) -> Result<Self, Self::Error> {
        Ok(Self::from_source_target(
            value.source.try_into()?,
            value.target.try_into()?,
        ))
    }
}

impl Converter {
    const IDENTITY: Converter = Converter {
        rotation: Quat::IDENTITY,
        matrix: Mat4::IDENTITY,
        remapping: RemappingConverter::IDENTITY,
    };

    pub(crate) fn from_source_target(source: ValidSemantics, target: ValidSemantics) -> Converter {
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

        // TODO: Mat4 to Quat is a bit brute force. Is there a better way?
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

    pub(crate) fn convert_translation(&self, source: Vec3) -> Vec3 {
        self.remapping.convert_translation(source)
    }

    pub(crate) fn convert_rotation(&self, source: Quat) -> Quat {
        source * self.rotation
    }

    pub(crate) fn convert_scale(&self, source: Vec3) -> Vec3 {
        self.remapping.convert_scale(source)
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct HierarchyConverter {
    // The converter to use for this node.
    local: Converter,

    // The converter that was used for the parent node.
    parent: Converter,
}

impl HierarchyConverter {
    pub(crate) fn from_local_and_parent(local: Converter, parent: Converter) -> Self {
        Self { local, parent }
    }

    pub(crate) fn local(&self) -> Converter {
        self.local
    }

    pub(crate) fn convert_translation(&self, t: Vec3) -> Vec3 {
        self.parent.convert_translation(t)
    }

    pub(crate) fn convert_rotation(&self, r: Quat) -> Quat {
        self.parent.rotation() * r * self.local.rotation().inverse()
    }

    pub(crate) fn convert_scale(&self, s: Vec3) -> Vec3 {
        self.local.convert_scale(s)
    }

    pub(crate) fn convert_transform(&self, t: Transform) -> Transform {
        Transform::from_translation(self.convert_translation(t.translation))
            .with_rotation(self.convert_rotation(t.rotation))
            .with_scale(self.convert_scale(t.scale))
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
                .map(|up| ValidSemantics::from_forward_up(*forward, *up).unwrap())
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

    // Test that our named semantics are valid.
    #[test]
    fn named_semantics() {
        assert_eq!(
            ValidSemantics::BEVY,
            ValidSemantics::try_from(Semantics::BEVY).unwrap()
        );

        assert_eq!(
            ValidSemantics::GLTF,
            ValidSemantics::try_from(Semantics::GLTF).unwrap()
        );
    }

    // A variety of directions generated from a semantics.
    #[derive(Debug)]
    struct TestDirections {
        forward: Vec3,
        up: Vec3,
        right: Vec3,
        diagonal: Vec3,
    }

    impl From<ValidSemantics> for TestDirections {
        fn from(value: ValidSemantics) -> Self {
            let forward = value.forward().into();
            let up = value.up().into();
            let right = value.right().into();
            let diagonal = forward + (2.0 * up) + (3.0 * right);

            TestDirections {
                forward,
                up,
                right,
                diagonal,
            }
        }
    }

    impl TestDirections {
        fn remapped(&self, converter: RemappingConverter) -> TestDirections {
            TestDirections {
                forward: converter.convert_translation(self.forward),
                up: converter.convert_translation(self.up),
                right: converter.convert_translation(self.right),
                diagonal: converter.convert_translation(self.diagonal),
            }
        }

        fn rotated(&self, converter: Quat) -> TestDirections {
            TestDirections {
                forward: converter * self.forward,
                up: converter * self.up,
                right: converter * self.right,
                diagonal: converter * self.diagonal,
            }
        }
    }

    impl PartialEq for TestDirections {
        fn eq(&self, other: &Self) -> bool {
            self.forward.abs_diff_eq(other.forward, 1e-6)
                && self.up.abs_diff_eq(other.up, 1e-6)
                && self.right.abs_diff_eq(other.right, 1e-6)
                && self.diagonal.abs_diff_eq(other.diagonal, 1e-6)
        }
    }

    #[test]
    // Generate directions from semantics and check that they match when
    // converted between semantics.
    fn converter() {
        for source_semantics in semantics_permutations() {
            for target_semantics in semantics_permutations() {
                let converter = Converter::from_source_target(source_semantics, target_semantics);
                let source = TestDirections::from(source_semantics);
                let target = TestDirections::from(target_semantics);

                let remapped = source.remapped(converter.remapping());
                let rotated = source.rotated(converter.rotation());

                assert_eq!(remapped, target);
                assert_eq!(rotated, target);
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

    impl Distribution<Converter> for RandomConverters {
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Converter {
            Converter::from_source_target(self.0.sample(rng), self.0.sample(rng))
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
                HierarchyConverter::from_local_and_parent(node_converters[0], Converter::IDENTITY),
                HierarchyConverter::from_local_and_parent(node_converters[1], node_converters[0]),
                // The leaf node uses the identity conversion, so a transform
                // in the leaf node's space and scene-space should be the same
                // regardless of how the other nodes in the hierarchy are
                // converted.
                HierarchyConverter::from_local_and_parent(Converter::IDENTITY, node_converters[1]),
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
