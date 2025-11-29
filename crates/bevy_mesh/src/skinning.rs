use bevy_asset::{AsAssetId, Asset, AssetId, Handle};
use bevy_ecs::{component::Component, entity::Entity, prelude::ReflectComponent, system::Query};
use bevy_math::{
    bounding::{Aabb3d, BoundingVolume},
    Affine3A, Mat4, Vec3, Vec3A,
};
use bevy_reflect::prelude::*;
use bevy_transform::components::GlobalTransform;
use core::ops::Deref;

use crate::{Mesh, MeshVertexAttribute, VertexAttributeValues, VertexFormat};

#[derive(Component, Debug, Default, Clone, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct SkinnedMesh {
    pub inverse_bindposes: Handle<SkinnedMeshInverseBindposes>,
    #[entities]
    pub joints: Vec<Entity>,
}

impl AsAssetId for SkinnedMesh {
    type Asset = SkinnedMeshInverseBindposes;

    // We implement this so that `AssetChanged` will work to pick up any changes
    // to `SkinnedMeshInverseBindposes`.
    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.inverse_bindposes.id()
    }
}

#[derive(Asset, TypePath, Debug)]
pub struct SkinnedMeshInverseBindposes(Box<[Mat4]>);

impl From<Vec<Mat4>> for SkinnedMeshInverseBindposes {
    fn from(value: Vec<Mat4>) -> Self {
        Self(value.into_boxed_slice())
    }
}

impl Deref for SkinnedMeshInverseBindposes {
    type Target = [Mat4];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// The AABB of a joint. This is optimized for `transform_aabb` - center/size is
// slightly faster than min/max, and the vectors don't benefit from alignment
// because they're broadcast loaded.
#[derive(Copy, Clone, Debug, PartialEq, Reflect)]
pub struct JointAabb {
    pub center: Vec3,
    pub half_size: Vec3,
}

impl From<JointAabb> for Aabb3d {
    fn from(value: JointAabb) -> Self {
        Self {
            min: Vec3A::from(value.center) - Vec3A::from(value.half_size),
            max: Vec3A::from(value.center) + Vec3A::from(value.half_size),
        }
    }
}

impl From<Aabb3d> for JointAabb {
    fn from(value: Aabb3d) -> Self {
        Self {
            center: value.center().into(),
            half_size: value.half_size().into(),
        }
    }
}

/// XXX TODO: Document.
#[derive(Clone, Default, Debug, Reflect, PartialEq)]
#[reflect(Clone)]
pub struct SkinnedMeshBounds {
    // Model-space AABBs that enclose the vertices skinned to a joint. This may
    // be a subset of the joints, as some might not be skinned to any vertices.
    //
    // `aabb_index_to_joint_index` maps from this array's indices to joint
    // indices.
    //
    // XXX TODO: Should be a Box<[PackedAabb3d]>, but that doesn't seem to work with reflection?
    pub aabbs: Vec<JointAabb>,

    // Maps from an `aabbs` array index to its joint index (`Mesh::ATTRIBUTE_JOINT_INDEX`).
    //
    // Caution: `aabbs` and `aabb_index_to_joint_index` should be the same
    // length. They're kept separate as a minor optimization - folding them into
    // one array would waste two bytes per joint due to alignment.
    //
    // XXX TODO: Should be a Box<[JointIndex]>, but that doesn't seem to work with reflection?
    pub aabb_index_to_joint_index: Vec<JointIndex>,
}

impl SkinnedMeshBounds {
    /// XXX TODO: Document.
    pub fn from_mesh(mesh: &Mesh) -> Result<SkinnedMeshBounds, MeshAttributeError> {
        let vertex_positions = expect_attribute_float32x3(mesh, Mesh::ATTRIBUTE_POSITION)?;
        let vertex_influences = InfluenceIterator::new(mesh)?;

        let Some(max_joint_index) = vertex_influences
            .clone()
            .map(|i| i.joint_index)
            .reduce(Ord::max)
        else {
            return Ok(SkinnedMeshBounds::default());
        };

        // Accumulate the AABB of each joint. Some joints may not have skinned
        // vertices, so their accumulators will be left empty.

        let mut accumulators: Box<[AabbAccumulator]> =
            vec![AabbAccumulator::new(); (max_joint_index as usize) + 1].into();

        for influence in vertex_influences {
            if let Some(&vertex_position) = vertex_positions.get(influence.vertex_index) {
                accumulators[influence.joint_index as usize]
                    .add_point(Vec3A::from_array(vertex_position));
            }
        }

        // Finish the accumulators and keep only joints with AABBs.

        let aabbs = accumulators
            .iter()
            .filter_map(|&accumulator| accumulator.finish().map(JointAabb::from))
            .collect::<Vec<_>>();

        let aabb_index_to_joint_index = accumulators
            .iter()
            .enumerate()
            .filter_map(|(joint_index, &accumulator)| {
                accumulator.finish().map(|_| joint_index as JointIndex)
            })
            .collect::<Vec<_>>();

        assert_eq!(aabbs.len(), aabb_index_to_joint_index.len());

        Ok(SkinnedMeshBounds {
            aabbs,
            aabb_index_to_joint_index,
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = (&JointIndex, &JointAabb)> {
        self.aabb_index_to_joint_index.iter().zip(self.aabbs.iter())
    }
}

#[derive(Copy, Clone, Debug)]
pub enum EntityAabbFromSkinnedMeshBoundsError {
    OutOfRangeJointIndex(JointIndex),
    MissingJointEntities,
    MissingSkinnedMeshBounds,
}

/// XXX TODO: Document.
pub fn entity_aabb_from_skinned_mesh_bounds(
    joint_entities: &Query<&GlobalTransform>,
    mesh: &Mesh,
    skinned_mesh: &SkinnedMesh,
    skinned_mesh_inverse_bindposes: &SkinnedMeshInverseBindposes,
    world_from_entity: Option<&GlobalTransform>,
) -> Result<Aabb3d, EntityAabbFromSkinnedMeshBoundsError> {
    let Some(skinned_mesh_bounds) = mesh.skinned_mesh_bounds() else {
        return Err(EntityAabbFromSkinnedMeshBoundsError::MissingSkinnedMeshBounds);
    };

    // Calculate an AABB that encloses the transformed AABBs of each joint.

    let mut accumulator = AabbAccumulator::new();

    for (&joint_index, &modelspace_joint_aabb) in skinned_mesh_bounds.iter() {
        let Some(joint_from_model) = skinned_mesh_inverse_bindposes
            .get(joint_index as usize)
            .map(|&m| Affine3A::from_mat4(m))
        else {
            return Err(EntityAabbFromSkinnedMeshBoundsError::OutOfRangeJointIndex(
                joint_index,
            ));
        };

        let Some(&joint_entity) = skinned_mesh.joints.get(joint_index as usize) else {
            return Err(EntityAabbFromSkinnedMeshBoundsError::OutOfRangeJointIndex(
                joint_index,
            ));
        };

        let Ok(&world_from_joint) = joint_entities.get(joint_entity) else {
            continue;
        };

        let world_from_model = world_from_joint.affine() * joint_from_model;
        let worldspace_joint_aabb = transform_aabb(modelspace_joint_aabb, world_from_model);

        accumulator.add_aabb(worldspace_joint_aabb);
    }

    let Some(worldspace_entity_aabb) = accumulator.finish() else {
        return Err(EntityAabbFromSkinnedMeshBoundsError::MissingJointEntities);
    };

    // If necessary, transform the AABB from world-space to entity-space.
    match world_from_entity {
        Some(world_from_entity) => Ok(transform_aabb(
            worldspace_entity_aabb.into(),
            world_from_entity.affine().inverse(),
        )),
        None => Ok(worldspace_entity_aabb),
    }
}

// Match the `Mesh` limits on joint indices (`ATTRIBUTE_JOINT_INDEX = VertexFormat::Uint16x4`)
//
// XXX TODO: Where should this go?
type JointIndex = u16;

// Return `(a * b) + c`, preferring fused multiply-adds if available.
fn fast_mul_add(a: Vec3A, b: Vec3A, c: Vec3A) -> Vec3A {
    #[cfg(target_feature = "fma")]
    return a.mul_add(b, c);
    #[cfg(not(target_feature = "fma"))]
    return (a * b) + c;
}

// Return the smallest AABB that encloses the transformed input AABB.
//
// Algorithm from "Transforming Axis-Aligned Bounding Boxes", James Arvo, Graphics Gems (1990).
#[inline]
fn transform_aabb(input: JointAabb, transform: Affine3A) -> Aabb3d {
    let mx = transform.matrix3.x_axis;
    let my = transform.matrix3.y_axis;
    let mz = transform.matrix3.z_axis;
    let mt = transform.translation;

    let cx = Vec3A::splat(input.center.x);
    let cy = Vec3A::splat(input.center.y);
    let cz = Vec3A::splat(input.center.z);

    let sx = Vec3A::splat(input.half_size.x);
    let sy = Vec3A::splat(input.half_size.y);
    let sz = Vec3A::splat(input.half_size.z);

    // Transform the center.
    let tc = fast_mul_add(mz, cz, mt);
    let tc = fast_mul_add(my, cy, tc);
    let tc = fast_mul_add(mx, cx, tc);

    // Calculate a size that encloses the transformed size.
    let ts = mx.abs() * sx;
    let ts = fast_mul_add(my.abs(), sy, ts);
    let ts = fast_mul_add(mz.abs(), sz, ts);

    let min = tc - ts;
    let max = tc + ts;

    Aabb3d { min, max }
}

// Helper for efficiently accumulating an enclosing AABB from a set of points or
// other AABBs. Intended for cases where the size of the set is not known in
// advance and might be zero.
//
// ```
// let a = AabbAccumulator::new();
//
// a.add_point(point); // Add a `Vec3A`.
// a.add_aabb(aabb); // Add an `Aabb3d`.
//
// // Returns `Some(Aabb3d)` if at least one thing was added.
// let result = a.finish();
// ```
//
// For alternatives, see [`Aabb3d::from_point_clound`](`bevy_math::bounding::bounded3d::Aabb3d::from_point_cloud`)
// and [`BoundingVolume::merge`](`bevy_math::bounding::BoundingVolume::merge`).
//
// XXX TODO: Maybe could move to `bevy_math`? Not sure if it's general purpose
// enough.
#[derive(Copy, Clone)]
struct AabbAccumulator {
    min: Vec3A,
    max: Vec3A,
}

impl AabbAccumulator {
    fn new() -> Self {
        // The initial state has `min > max`, and the first add will make
        // `min <= max`. This means `finish` can detect if nothing was added,
        // and the add functions can be branchless.
        Self {
            min: Vec3A::MAX,
            max: Vec3A::MIN,
        }
    }

    fn add_aabb(&mut self, aabb: Aabb3d) {
        self.min = self.min.min(aabb.min);
        self.max = self.max.max(aabb.max);
    }

    fn add_point(&mut self, position: Vec3A) {
        self.min = self.min.min(position);
        self.max = self.max.max(position);
    }

    /// Returns the enclosing AABB if at least one thing was added, otherwise `None`.
    fn finish(self) -> Option<Aabb3d> {
        if self.min.cmpgt(self.max).any() {
            None
        } else {
            Some(Aabb3d {
                min: self.min,
                max: self.max,
            })
        }
    }
}

/// A single vertex influence. Used by [`InfluenceIterator`].
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Influence {
    pub vertex_index: usize,
    pub joint_index: JointIndex,
    pub joint_weight: f32,
}

/// Iterator over all vertex influences with non-zero weight.
#[derive(Clone, Debug)]
pub struct InfluenceIterator<'a> {
    vertex_count: usize,
    joint_indices: &'a [[u16; 4]],
    joint_weights: &'a [[f32; 4]],
    vertex_index: usize,
    influence_index: usize,
}

impl<'a> InfluenceIterator<'a> {
    pub fn new(mesh: &'a Mesh) -> Result<Self, MeshAttributeError> {
        let joint_indices = expect_attribute_uint16x4(mesh, Mesh::ATTRIBUTE_JOINT_INDEX)?;
        let joint_weights = expect_attribute_float32x4(mesh, Mesh::ATTRIBUTE_JOINT_WEIGHT)?;

        Ok(InfluenceIterator {
            vertex_count: joint_indices.len().min(joint_weights.len()),
            joint_indices,
            joint_weights,
            vertex_index: 0,
            influence_index: 0,
        })
    }
    // `Mesh` only supports four influences, so we can make this const for
    // simplicity. If `Mesh` gains support for variable influences then this
    // will become a variable.
    const MAX_INFLUENCES: usize = 4;
}

impl Iterator for InfluenceIterator<'_> {
    type Item = Influence;

    fn next(&mut self) -> Option<Influence> {
        loop {
            assert!(self.influence_index <= Self::MAX_INFLUENCES);
            assert!(self.vertex_index <= self.vertex_count);

            if self.influence_index >= Self::MAX_INFLUENCES {
                self.influence_index = 0;
                self.vertex_index += 1;
            }

            if self.vertex_index >= self.vertex_count {
                return None;
            }

            let joint_index = self.joint_indices[self.vertex_index][self.influence_index];
            let joint_weight = self.joint_weights[self.vertex_index][self.influence_index];

            self.influence_index += 1;

            if joint_weight > 0.0 {
                return Some(Influence {
                    vertex_index: self.vertex_index,
                    joint_index,
                    joint_weight,
                });
            }
        }
    }
}

/// Generic error for when a mesh was expected to have a certain attribute with
/// a certain type.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MeshAttributeError {
    MissingAttribute(&'static str),
    UnexpectedFormat(&'static str, VertexFormat),
}

// Implements a function that returns a mesh attribute's data or `MeshAttributeError`.
//
// `impl_expect_attribute!(expect_attribute_float32x3, Float32x3, [f32; 3])` would
// implement `fn expect_attribute_float32x3(mesh: &Mesh) -> Result<Vec<[f32; 3]>, MeshAttributeError`.
macro_rules! impl_expect_attribute {
    ($name:ident, $value_type:ident, $output_type:ty) => {
        fn $name<'a>(
            mesh: &'a Mesh,
            attribute: MeshVertexAttribute,
        ) -> Result<&'a Vec<$output_type>, MeshAttributeError> {
            match mesh.attribute(attribute) {
                Some(VertexAttributeValues::$value_type(v)) => Ok(v),
                Some(v) => {
                    return Err(MeshAttributeError::UnexpectedFormat(
                        attribute.name,
                        v.into(),
                    ))
                }
                None => return Err(MeshAttributeError::MissingAttribute(attribute.name)),
            }
        }
    };
}

impl_expect_attribute!(expect_attribute_float32x3, Float32x3, [f32; 3]);
impl_expect_attribute!(expect_attribute_float32x4, Float32x4, [f32; 4]);
impl_expect_attribute!(expect_attribute_uint16x4, Uint16x4, [u16; 4]);

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use bevy_asset::RenderAssetUsages;
    use bevy_math::{bounding::BoundingVolume, vec3, vec3a};

    #[test]
    fn aabb_accumulator() {
        assert_eq!(AabbAccumulator::new().finish(), None);

        let nice_aabbs = &[
            Aabb3d {
                min: vec3a(1.0, 2.0, 3.0),
                max: vec3a(5.0, 4.0, 3.0),
            },
            Aabb3d {
                min: vec3a(-99.0, 2.0, 3.0),
                max: vec3a(5.0, 4.0, 3.0),
            },
            Aabb3d {
                min: vec3a(1.0, 2.0, 3.0),
                max: vec3a(5.0, 99.0, 3.0),
            },
        ];

        let naughty_aabbs = &[
            Aabb3d {
                min: Vec3A::MIN,
                max: Vec3A::MAX,
            },
            Aabb3d {
                min: Vec3A::MIN,
                max: Vec3A::MIN,
            },
            Aabb3d {
                min: Vec3A::MAX,
                max: Vec3A::MAX,
            },
        ];

        for aabbs in [nice_aabbs, naughty_aabbs] {
            for &aabb in aabbs {
                let point = aabb.min;

                let mut one_aabb = AabbAccumulator::new();
                let mut one_point = AabbAccumulator::new();

                one_aabb.add_aabb(aabb);
                one_point.add_point(point);

                assert_eq!(one_aabb.finish(), Some(aabb));
                assert_eq!(
                    one_point.finish(),
                    Some(Aabb3d {
                        min: point,
                        max: point
                    })
                );
            }

            {
                let mut multiple_aabbs = AabbAccumulator::new();
                let mut multiple_points = AabbAccumulator::new();

                for &aabb in aabbs {
                    multiple_aabbs.add_aabb(aabb);
                    multiple_points.add_point(aabb.min);
                    multiple_points.add_point(aabb.max);
                }

                let expected = aabbs.iter().cloned().reduce(|l, r| l.merge(&r));

                assert_eq!(multiple_aabbs.finish(), expected);
                assert_eq!(multiple_points.finish(), expected);
            }
        }
    }

    #[test]
    fn influence_iterator() {
        let mesh = Mesh::new(
            wgpu_types::PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );

        assert_eq!(
            InfluenceIterator::new(&mesh).err(),
            Some(MeshAttributeError::MissingAttribute(
                Mesh::ATTRIBUTE_JOINT_INDEX.name
            ))
        );

        let mesh = mesh.with_inserted_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            VertexAttributeValues::Uint16x4(vec![
                [1, 0, 0, 0],
                [0, 2, 0, 0],
                [0, 0, 3, 0],
                [0, 0, 0, 4],
                [1, 2, 0, 0],
                [3, 4, 5, 0],
                [6, 7, 8, 9],
            ]),
        );

        assert_eq!(
            InfluenceIterator::new(&mesh).err(),
            Some(MeshAttributeError::MissingAttribute(
                Mesh::ATTRIBUTE_JOINT_WEIGHT.name
            ))
        );

        let mesh = mesh.with_inserted_attribute(
            Mesh::ATTRIBUTE_JOINT_WEIGHT,
            VertexAttributeValues::Float32x4(vec![
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
                [0.1, 0.9, 0.0, 0.0],
                [0.1, 0.2, 0.7, 0.0],
                [0.1, 0.2, 0.4, 0.3],
            ]),
        );

        let expected = &[
            Influence {
                vertex_index: 0,
                joint_index: 1,
                joint_weight: 1.0,
            },
            Influence {
                vertex_index: 1,
                joint_index: 2,
                joint_weight: 1.0,
            },
            Influence {
                vertex_index: 2,
                joint_index: 3,
                joint_weight: 1.0,
            },
            Influence {
                vertex_index: 3,
                joint_index: 4,
                joint_weight: 1.0,
            },
            Influence {
                vertex_index: 4,
                joint_index: 1,
                joint_weight: 0.1,
            },
            Influence {
                vertex_index: 4,
                joint_index: 2,
                joint_weight: 0.9,
            },
            Influence {
                vertex_index: 5,
                joint_index: 3,
                joint_weight: 0.1,
            },
            Influence {
                vertex_index: 5,
                joint_index: 4,
                joint_weight: 0.2,
            },
            Influence {
                vertex_index: 5,
                joint_index: 5,
                joint_weight: 0.7,
            },
            Influence {
                vertex_index: 6,
                joint_index: 6,
                joint_weight: 0.1,
            },
            Influence {
                vertex_index: 6,
                joint_index: 7,
                joint_weight: 0.2,
            },
            Influence {
                vertex_index: 6,
                joint_index: 8,
                joint_weight: 0.4,
            },
            Influence {
                vertex_index: 6,
                joint_index: 9,
                joint_weight: 0.3,
            },
        ];

        assert_eq!(
            InfluenceIterator::new(&mesh).unwrap().collect::<Vec<_>>(),
            expected
        );
    }

    fn aabb_assert_eq(a: Aabb3d, b: Aabb3d) {
        assert_abs_diff_eq!(a.min.x, b.min.x);
        assert_abs_diff_eq!(a.min.y, b.min.y);
        assert_abs_diff_eq!(a.min.z, b.min.z);
        assert_abs_diff_eq!(a.max.x, b.max.x);
        assert_abs_diff_eq!(a.max.y, b.max.y);
        assert_abs_diff_eq!(a.max.z, b.max.z);
    }

    // Like `transform_aabb`, but uses the naive method of calculating each corner.
    fn alternative_transform_aabb(input: JointAabb, transform: Affine3A) -> Aabb3d {
        let c = input.center;
        let s = input.half_size;

        let corners = [
            vec3(c.x + s.x, c.y + s.y, c.z + s.z),
            vec3(c.x - s.x, c.y + s.y, c.z + s.z),
            vec3(c.x + s.x, c.y - s.y, c.z + s.z),
            vec3(c.x - s.x, c.y - s.y, c.z + s.z),
            vec3(c.x + s.x, c.y + s.y, c.z - s.z),
            vec3(c.x - s.x, c.y + s.y, c.z - s.z),
            vec3(c.x + s.x, c.y - s.y, c.z - s.z),
            vec3(c.x - s.x, c.y - s.y, c.z - s.z),
        ];

        let mut a = AabbAccumulator::new();

        for corner in corners {
            a.add_point(transform.transform_point3(corner).into());
        }

        a.finish().unwrap()
    }

    #[test]
    fn transform_aabb() {
        let aabbs = [
            JointAabb {
                center: Vec3::ZERO,
                half_size: Vec3::ZERO,
            },
            JointAabb {
                center: Vec3::ZERO,
                half_size: vec3(2.0, 3.0, 4.0),
            },
            JointAabb {
                center: vec3(2.0, 3.0, 4.0),
                half_size: Vec3::ZERO,
            },
            JointAabb {
                center: vec3(20.0, -30.0, 40.0),
                half_size: vec3(5.0, 6.0, 7.0),
            },
        ];

        // Various transforms, including awkward ones like skews and
        // negative/zero scales.
        let transforms = [
            Affine3A::IDENTITY,
            Affine3A::from_cols(Vec3A::X, Vec3A::Z, Vec3A::Y, vec3a(1.0, 2.0, 3.0)),
            Affine3A::from_cols(Vec3A::Y, Vec3A::X, Vec3A::Z, vec3a(1.0, 2.0, 3.0)),
            Affine3A::from_cols(Vec3A::Z, Vec3A::Y, Vec3A::X, vec3a(1.0, 2.0, 3.0)),
            Affine3A::from_scale(Vec3::ZERO),
            Affine3A::from_scale(vec3(2.0, 3.0, 4.0)),
            Affine3A::from_scale(vec3(-2.0, 3.0, -4.0)),
            Affine3A::from_cols(
                vec3a(1.0, 2.0, -3.0),
                vec3a(4.0, -5.0, 6.0),
                vec3a(-7.0, 8.0, 9.0),
                vec3a(1.0, -2.0, 3.0),
            ),
        ];

        for aabb in aabbs {
            for transform in transforms {
                aabb_assert_eq(
                    super::transform_aabb(aabb, transform),
                    alternative_transform_aabb(aabb, transform),
                );
            }
        }
    }
}
