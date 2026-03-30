use crate::{
    Mesh, Mesh3d, MeshAccessError, MeshVertexAttribute, VertexAttributeValues, VertexFormat,
};
use bevy_asset::{AsAssetId, Asset, AssetId, Assets, Handle};
use bevy_ecs::{component::Component, entity::Entity, prelude::ReflectComponent, system::Query};
use bevy_math::{
    bounding::{Aabb3d, BoundingVolume},
    Affine3A, Mat3A, Mat4, Vec3, Vec3A, Vec4,
};
use bevy_reflect::prelude::*;
use bevy_transform::components::GlobalTransform;
use core::ops::Deref;
use thiserror::Error;

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
// slightly faster than the min/max used by `bevy_math::Aabb3d`, and the vectors
// don't benefit from alignment because they're broadcast loaded.
#[derive(Copy, Clone, Debug, PartialEq, Reflect)]
pub struct JointAabb {
    pub center: Vec3,
    pub half_size: Vec3,
}

impl JointAabb {
    fn min(&self) -> Vec3 {
        self.center - self.half_size
    }

    fn max(&self) -> Vec3 {
        self.center + self.half_size
    }
}

impl From<JointAabb> for Aabb3d {
    fn from(value: JointAabb) -> Self {
        Self {
            min: value.min().into(),
            max: value.max().into(),
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

/// Data that can be used to calculate the AABB of a skinned mesh.
#[derive(Clone, Default, Debug, PartialEq, Reflect)]
#[reflect(Clone)]
pub struct SkinnedMeshBounds {
    // Model-space AABBs that enclose the vertices skinned to a joint. Some
    // joints may not be skinned to any vertices, so not every joint has an
    // AABB.
    //
    // `aabb_index_to_joint_index` maps from an `aabbs` index to a joint index,
    // which corresponds to `Mesh::ATTRIBUTE_JOINT_INDEX` and `SkinnedMesh::joints`.
    //
    // These arrays could be a single `Vec<(JointAabb, JointIndex)>`, but that
    // would waste two bytes due to alignment.
    //
    // TODO: If https://github.com/bevyengine/bevy/issues/11570 is fixed, `Vec<_>`
    // can be changed to `Box<[_]>`.
    pub aabbs: Vec<JointAabb>,
    pub aabb_index_to_joint_index: Vec<JointIndex>,
}

#[derive(Clone, PartialEq, Debug, Error)]
pub enum SkinnedMeshBoundsError {
    #[error("The mesh does not contain any joints that are skinned to vertices")]
    NoSkinnedJoints,
    #[error(transparent)]
    MeshAttributeError(#[from] MeshAttributeError),
}

impl SkinnedMeshBounds {
    /// Create a `SkinnedMeshBounds` from a [`Mesh`].
    ///
    /// The mesh is expected to have position, joint index and joint weight
    /// attributes. If any are missing then a [`MeshAttributeError`] is returned.
    pub fn from_mesh(mesh: &Mesh) -> Result<SkinnedMeshBounds, SkinnedMeshBoundsError> {
        let vertex_positions = expect_attribute_float32x3(mesh, Mesh::ATTRIBUTE_POSITION)?;
        let vertex_influences = InfluenceIterator::new(mesh)?;

        // Find the maximum joint index.
        let Some(max_joint_index) = vertex_influences
            .clone()
            .map(|i| i.joint_index.0 as usize)
            .reduce(Ord::max)
        else {
            return Ok(SkinnedMeshBounds::default());
        };

        // Create an AABB accumulator for each joint.
        let mut accumulators: Box<[AabbAccumulator]> =
            vec![AabbAccumulator::new(); max_joint_index + 1].into();

        // Iterate over all vertex influences and add the vertex position to
        // the influencing joint's AABB.
        for influence in vertex_influences {
            if let Some(&vertex_position) = vertex_positions.get(influence.vertex_index) {
                accumulators[influence.joint_index.0 as usize]
                    .add_point(Vec3A::from_array(vertex_position));
            }
        }

        // Filter out joints with no AABB.
        let joint_indices_and_aabbs = accumulators
            .iter()
            .enumerate()
            .filter_map(|(joint_index, &accumulator)| {
                accumulator.finish().map(|aabb| (joint_index, aabb))
            })
            .collect::<Vec<_>>();

        if joint_indices_and_aabbs.is_empty() {
            return Err(SkinnedMeshBoundsError::NoSkinnedJoints);
        }

        let aabbs = joint_indices_and_aabbs
            .iter()
            .map(|&(_, aabb)| JointAabb::from(aabb))
            .collect::<Vec<_>>();

        let aabb_index_to_joint_index = joint_indices_and_aabbs
            .iter()
            .map(|&(joint_index, _)| JointIndex(joint_index as u16))
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
    MissingJointEntity,
    MissingSkinnedMeshBounds,
}

/// Given the components of a skinned mesh entity, return an `Aabb3d` that
/// encloses the skinned vertices of the mesh.
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

    let mut accumulator = AabbAccumulator::new();

    // For each model-space joint AABB, transform it to world-space and add it
    // to the accumulator.
    for (&joint_index, &modelspace_joint_aabb) in skinned_mesh_bounds.iter() {
        let Some(joint_from_model) = skinned_mesh_inverse_bindposes
            .get(joint_index.0 as usize)
            .map(|&m| Affine3A::from_mat4(m))
        else {
            return Err(EntityAabbFromSkinnedMeshBoundsError::OutOfRangeJointIndex(
                joint_index,
            ));
        };

        let Some(&joint_entity) = skinned_mesh.joints.get(joint_index.0 as usize) else {
            return Err(EntityAabbFromSkinnedMeshBoundsError::OutOfRangeJointIndex(
                joint_index,
            ));
        };

        let Ok(&world_from_joint) = joint_entities.get(joint_entity) else {
            return Err(EntityAabbFromSkinnedMeshBoundsError::MissingJointEntity);
        };

        let world_from_model = world_from_joint.affine() * joint_from_model;
        let worldspace_joint_aabb = transform_aabb(modelspace_joint_aabb, world_from_model);

        accumulator.add_aabb(worldspace_joint_aabb);
    }

    let Some(worldspace_entity_aabb) = accumulator.finish() else {
        return Err(EntityAabbFromSkinnedMeshBoundsError::MissingJointEntity);
    };

    // If the entity has a transform, move the AABB from world-space to entity-space.
    if let Some(world_from_entity) = world_from_entity {
        let entityspace_entity_aabb = transform_aabb(
            worldspace_entity_aabb.into(),
            world_from_entity.affine().inverse(),
        );

        Ok(entityspace_entity_aabb)
    } else {
        Ok(worldspace_entity_aabb)
    }
}

// Return the smallest `Aabb3d` that encloses the transformed `JointAabb`.
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
    let tc = (mx * cx) + (my * cy) + (mz * cz) + mt;

    // Calculate a size that encloses the transformed size.
    let ts = (mx.abs() * sx) + (my.abs() * sy) + (mz.abs() * sz);

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
#[derive(Copy, Clone)]
struct AabbAccumulator {
    min: Vec3A,
    max: Vec3A,
}

impl AabbAccumulator {
    fn new() -> Self {
        // Initialize in such a way that adds can be branchless but `finish` can
        // still detect if nothing was added. The initial state has `min > max`,
        // but the first add will make `min <= max`.
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

// An index that corresponds to `Mesh::ATTRIBUTE_JOINT_INDEX` and `SkinnedMesh::joints`.
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
pub struct JointIndex(pub u16);

// The number of influences contained in `Mesh::ATTRIBUTE_JOINT_INDEX` and
// `Mesh::ATTRIBUTE_JOINT_WEIGHT`.
const INFLUENCE_COUNT: usize = 4;

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
    joint_indices: &'a [[u16; INFLUENCE_COUNT]],
    joint_weights: &'a [[f32; INFLUENCE_COUNT]],
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
}

impl Iterator for InfluenceIterator<'_> {
    type Item = Influence;

    fn next(&mut self) -> Option<Influence> {
        loop {
            assert!(self.influence_index <= INFLUENCE_COUNT);
            assert!(self.vertex_index <= self.vertex_count);

            if self.influence_index >= INFLUENCE_COUNT {
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
                    joint_index: JointIndex(joint_index),
                    joint_weight,
                });
            }
        }
    }
}

/// Generic error for when a mesh was expected to have a certain attribute with
/// a certain format.
#[derive(Clone, PartialEq, Debug, Error)]
pub enum MeshAttributeError {
    #[error("Missing attribute \"{0}\"")]
    MissingAttribute(&'static str),
    #[error("Attribute \"{0}\" has unexpected format {1:?}")]
    UnexpectedFormat(&'static str, VertexFormat),
    #[error(transparent)]
    MeshAccessError(#[from] MeshAccessError),
}

// Implements a function that returns a mesh attribute's data or `MeshAttributeError`.
//
// ```
// impl_expect_attribute!(expect_attribute_float32x3, Float32x3, [f32; 3]);
//
// let positions: &[[f32; 3]] = expect_attribute_float32x3(mesh, Mesh::ATTRIBUTE_POSITION)?;
// ```
macro_rules! impl_expect_attribute {
    ($name:ident, $value_type:ident, $output_type:ty) => {
        fn $name<'a>(
            mesh: &'a Mesh,
            attribute: MeshVertexAttribute,
        ) -> Result<&'a [$output_type], MeshAttributeError> {
            match mesh.try_attribute_option(attribute)? {
                Some(VertexAttributeValues::$value_type(v)) => Ok(&v),
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

// Multiply an `Affine3A` by the given weight.
fn weight_affine3(affine: Affine3A, weight: f32) -> Affine3A {
    Affine3A::from_cols(
        weight * affine.matrix3.x_axis,
        weight * affine.matrix3.y_axis,
        weight * affine.matrix3.z_axis,
        weight * affine.translation,
    )
}

// Add two `Affine3A`s.
fn add_affine3(l: Affine3A, r: Affine3A) -> Affine3A {
    Affine3A::from_cols(
        l.matrix3.x_axis + r.matrix3.x_axis,
        l.matrix3.y_axis + r.matrix3.y_axis,
        l.matrix3.z_axis + r.matrix3.z_axis,
        l.translation + r.translation,
    )
}

// Sum a slice of `Affine3A`s.
fn sum_affine3(affines: &[Affine3A]) -> Affine3A {
    affines
        .iter()
        .copied()
        .reduce(add_affine3)
        .unwrap_or(Affine3A::IDENTITY)
}

// Matches the `inverse_transpose_3x3m` function in `bevy_pbr/skinning.wgsl`.
fn inverse_transpose_3x3m(m: Mat3A) -> Mat3A {
    let x = m.y_axis.cross(m.z_axis);
    let y = m.z_axis.cross(m.x_axis);
    let z = m.x_axis.cross(m.y_axis);
    let det = m.z_axis.dot(z);
    Mat3A::from_cols(x / det, y / det, z / det)
}

// Matches the `skin_normals` function in `bevy_pbr/skinning.wgsl`.
fn skin_normal(transform: &Affine3A, normal: Vec3A) -> Vec3A {
    (inverse_transpose_3x3m(transform.matrix3) * normal).normalize()
}

// Based on the `mesh_tangent_local_to_world` function in
// `bevy_pbr/mesh_functions.wgsl`, but we don't need to sign flip from the
// mesh-to-world determinant because we're skinning in mesh space.
fn skin_tangent(transform: &Affine3A, tangent: Vec4) -> Vec4 {
    if tangent == Vec4::ZERO {
        tangent
    } else {
        (transform.matrix3 * tangent.truncate())
            .normalize()
            .extend(tangent.w)
    }
}

// Check that an attribute has the expected length, returning
// `SkinMeshError::MismatchedMeshAttributeLengths` if not.
fn check_attribute_length<T>(expected: usize, actual: Option<&[T]>) -> Result<(), SkinMeshError> {
    if actual.is_some_and(|actual| expected != actual.len()) {
        Err(SkinMeshError::MismatchedMeshAttributeLengths)
    } else {
        Ok(())
    }
}

#[derive(Clone, PartialEq, Debug, Error)]
pub enum SkinMeshError {
    #[error("The `Mesh` asset was not found")]
    MissingMeshAsset,
    #[error("The `SkinnedMeshInverseBindposes` asset was not found")]
    MissingInverseBindposesAsset,
    #[error("A joint entity referenced by `SkinnedMesh::joints` was not found")]
    MissingJointEntity,
    #[error("A vertex's `Mesh::ATTRIBUTE_JOINT_INDEX` had an out of range joint index, or the `SkinnedMesh::joints` array was the wrong length")]
    InvalidJointIndex,
    #[error("`SkinnedMesh::joints` and `SkinnedMeshInverseBindposes` were different lengths")]
    MismatchedJointAndInverseBindposesLengths,
    #[error("Mesh attributes were not all the same length")]
    MismatchedMeshAttributeLengths,
    #[error(transparent)]
    MeshAttributeError(#[from] MeshAttributeError),
}

impl From<MeshAccessError> for SkinMeshError {
    fn from(value: MeshAccessError) -> Self {
        Self::MeshAttributeError(MeshAttributeError::MeshAccessError(value))
    }
}

// Skin the given mesh attributes in-place.
fn skin_vertices(
    mut positions: Option<&mut [[f32; 3]]>,
    mut normals: Option<&mut [[f32; 3]]>,
    mut tangents: Option<&mut [[f32; 4]]>,
    joint_indices: &[[u16; INFLUENCE_COUNT]],
    joint_weights: &[[f32; INFLUENCE_COUNT]],
    mesh_from_mesh_bindpose_array: &[Affine3A],
) -> Result<(), SkinMeshError> {
    let vertex_count = joint_indices.len();

    check_attribute_length(vertex_count, Some(joint_weights))?;
    check_attribute_length(vertex_count, positions.as_deref())?;
    check_attribute_length(vertex_count, normals.as_deref())?;
    check_attribute_length(vertex_count, tangents.as_deref())?;

    for vertex_index in 0..vertex_count {
        let vertex_joint_indices = joint_indices[vertex_index];
        let vertex_joint_weights = joint_weights[vertex_index];

        // The weighted transform of each joint influence.
        let mut transforms = [Affine3A::ZERO; INFLUENCE_COUNT];

        for influence in 0..INFLUENCE_COUNT {
            let joint_index = vertex_joint_indices[influence] as usize;
            let joint_weight = vertex_joint_weights[influence];
            let mesh_from_mesh_bindpose = *mesh_from_mesh_bindpose_array
                .get(joint_index)
                .ok_or(SkinMeshError::InvalidJointIndex)?;

            transforms[influence] = weight_affine3(mesh_from_mesh_bindpose, joint_weight);
        }

        // The final transform is the sum of the weighted joint transforms.
        // Matches the `skin_model` function in `bevy_pbr/skinning.wgsl`.
        let transform = sum_affine3(&transforms);

        if let Some(positions) = positions.as_deref_mut() {
            let unskinned = Vec3A::from_slice(&positions[vertex_index]);
            let skinned = transform.transform_point3a(unskinned);
            (*positions)[vertex_index] = <[f32; 3]>::from(skinned);
        }

        if let Some(normals) = normals.as_deref_mut() {
            let unskinned = Vec3A::from_slice(&normals[vertex_index]);
            let skinned = skin_normal(&transform, unskinned);
            (*normals)[vertex_index] = <[f32; 3]>::from(skinned);
        }

        if let Some(tangents) = tangents.as_deref_mut() {
            let unskinned = Vec4::from_slice(&tangents[vertex_index]);
            let skinned = skin_tangent(&transform, unskinned);
            (*tangents)[vertex_index] = <[f32; 4]>::from(skinned);
        }
    }

    Ok(())
}

/// Given a skinned mesh and joint transforms, return a copy of the mesh with
/// skinning applied to the vertex positions, normals and tangents.
///
/// The given joint transforms map from the joint's bind-pose transform in
/// mesh-space to the joint's target transform in mesh-space. This is typically
/// calculated from the mesh's world position, the joint's world position, and
/// the joint's inverse bind-pose:
/// `mesh_from_mesh_bindpose = mesh_from_world * world_from_joint * inverse_bindpose`.
///
/// The returned mesh preserves the input mesh's settings (like
/// `primitive_topology`) and attributes unrelated to skinning (like UVs). It
/// will not contain joint index or weight attributes, morph targets, or data
/// derived from attributes (like the AABB).
pub fn skin_mesh(
    input_mesh: &Mesh,
    mesh_from_mesh_bindpose_array: &[Affine3A],
) -> Result<Mesh, SkinMeshError> {
    let joint_indices = expect_attribute_uint16x4(input_mesh, Mesh::ATTRIBUTE_JOINT_INDEX)?;
    let joint_weights = expect_attribute_float32x4(input_mesh, Mesh::ATTRIBUTE_JOINT_WEIGHT)?;

    let mut positions = match input_mesh
        .try_attribute_option(Mesh::ATTRIBUTE_POSITION.id)?
        .cloned()
    {
        Some(VertexAttributeValues::Float32x3(values)) => Some(values),
        Some(_) => Err(MeshAttributeError::UnexpectedFormat(
            Mesh::ATTRIBUTE_POSITION.name,
            Mesh::ATTRIBUTE_POSITION.format,
        ))?,
        None => None,
    };

    let mut normals = match input_mesh
        .try_attribute_option(Mesh::ATTRIBUTE_NORMAL.id)?
        .cloned()
    {
        Some(VertexAttributeValues::Float32x3(values)) => Some(values),
        Some(_) => Err(MeshAttributeError::UnexpectedFormat(
            Mesh::ATTRIBUTE_NORMAL.name,
            Mesh::ATTRIBUTE_NORMAL.format,
        ))?,
        None => None,
    };

    let mut tangents = match input_mesh
        .try_attribute_option(Mesh::ATTRIBUTE_TANGENT.id)?
        .cloned()
    {
        Some(VertexAttributeValues::Float32x4(values)) => Some(values),
        Some(_) => Err(MeshAttributeError::UnexpectedFormat(
            Mesh::ATTRIBUTE_TANGENT.name,
            Mesh::ATTRIBUTE_TANGENT.format,
        ))?,
        None => None,
    };

    skin_vertices(
        positions.as_deref_mut(),
        normals.as_deref_mut(),
        tangents.as_deref_mut(),
        joint_indices,
        joint_weights,
        mesh_from_mesh_bindpose_array,
    )?;

    // Start with an empty copy of the input mesh.
    let mut output_mesh = input_mesh.as_empty();

    if let Some(indices) = input_mesh.indices() {
        output_mesh.insert_indices(indices.clone());
    }

    // Copy attributes from the input mesh. This excludes the position/normal/tangent
    // attributes that will be replaced with our skinned versions, and the joint
    // index/weight attributes that are now redundant.
    for (&attribute, values) in input_mesh.attributes() {
        if (attribute != Mesh::ATTRIBUTE_POSITION)
            && (attribute != Mesh::ATTRIBUTE_NORMAL)
            && (attribute != Mesh::ATTRIBUTE_TANGENT)
            && (attribute != Mesh::ATTRIBUTE_JOINT_WEIGHT)
            && (attribute != Mesh::ATTRIBUTE_JOINT_INDEX)
        {
            output_mesh.insert_attribute(attribute, values.clone());
        }
    }

    if let Some(positions) = positions {
        output_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    }

    if let Some(normals) = normals {
        output_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    }

    if let Some(tangents) = tangents {
        output_mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
    }

    Ok(output_mesh)
}

/// Given the components of a skinned mesh entity, return a copy of the mesh
/// asset with skinning applied to the vertex positions, normals and tangents.
///
/// The returned mesh preserves the input mesh's settings (like
/// `primitive_topology`) and attributes unrelated to skinning (like UVs). It
/// will not contain joint index or weight attributes, morph targets, or data
/// derived from attributes (like the AABB).
pub fn skin_mesh_entity(
    mesh_component: &Mesh3d,
    skinned_mesh_component: &SkinnedMesh,
    world_from_mesh: &GlobalTransform,
    mesh_assets: &Assets<Mesh>,
    inverse_bindposes_assets: &Assets<SkinnedMeshInverseBindposes>,
    world_from_joint_query: &Query<&GlobalTransform>,
) -> Result<Mesh, SkinMeshError> {
    let input_mesh = mesh_assets
        .get(&mesh_component.0)
        .ok_or(SkinMeshError::MissingMeshAsset)?;

    let inverse_bindposes = inverse_bindposes_assets
        .get(&skinned_mesh_component.inverse_bindposes)
        .ok_or(SkinMeshError::MissingInverseBindposesAsset)?;

    if skinned_mesh_component.joints.len() != inverse_bindposes.len() {
        return Err(SkinMeshError::MismatchedJointAndInverseBindposesLengths);
    }

    let mesh_from_world = world_from_mesh.affine().inverse();

    let mesh_from_joint_array = skinned_mesh_component
        .joints
        .iter()
        .map(|&joint| {
            let world_from_joint = world_from_joint_query.get(joint).ok()?.affine();
            Some(mesh_from_world * world_from_joint)
        })
        .collect::<Option<Vec<_>>>()
        .ok_or(SkinMeshError::MissingJointEntity)?;

    let mesh_from_mesh_bindpose_array = mesh_from_joint_array
        .into_iter()
        .zip(inverse_bindposes.iter())
        .map(|(mesh_from_joint, &joint_from_mesh_bindpose)| {
            mesh_from_joint * Affine3A::from_mat4(joint_from_mesh_bindpose)
        })
        .collect::<Vec<_>>();

    skin_mesh(input_mesh, &mesh_from_mesh_bindpose_array)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Indices;
    use approx::assert_abs_diff_eq;
    use bevy_asset::RenderAssetUsages;
    use bevy_math::{bounding::BoundingVolume, vec3, vec3a};
    use wgpu_types::PrimitiveTopology;

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
            PrimitiveTopology::TriangleList,
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
                joint_index: JointIndex(1),
                joint_weight: 1.0,
            },
            Influence {
                vertex_index: 1,
                joint_index: JointIndex(2),
                joint_weight: 1.0,
            },
            Influence {
                vertex_index: 2,
                joint_index: JointIndex(3),
                joint_weight: 1.0,
            },
            Influence {
                vertex_index: 3,
                joint_index: JointIndex(4),
                joint_weight: 1.0,
            },
            Influence {
                vertex_index: 4,
                joint_index: JointIndex(1),
                joint_weight: 0.1,
            },
            Influence {
                vertex_index: 4,
                joint_index: JointIndex(2),
                joint_weight: 0.9,
            },
            Influence {
                vertex_index: 5,
                joint_index: JointIndex(3),
                joint_weight: 0.1,
            },
            Influence {
                vertex_index: 5,
                joint_index: JointIndex(4),
                joint_weight: 0.2,
            },
            Influence {
                vertex_index: 5,
                joint_index: JointIndex(5),
                joint_weight: 0.7,
            },
            Influence {
                vertex_index: 6,
                joint_index: JointIndex(6),
                joint_weight: 0.1,
            },
            Influence {
                vertex_index: 6,
                joint_index: JointIndex(7),
                joint_weight: 0.2,
            },
            Influence {
                vertex_index: 6,
                joint_index: JointIndex(8),
                joint_weight: 0.4,
            },
            Influence {
                vertex_index: 6,
                joint_index: JointIndex(9),
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

    // Like `transform_aabb`, but uses the naive method of transforming each corner.
    fn naive_transform_aabb(input: JointAabb, transform: Affine3A) -> Aabb3d {
        let minmax = [input.min(), input.max()];

        let mut accumulator = AabbAccumulator::new();

        for i in 0..8 {
            let corner = vec3(
                minmax[i & 1].x,
                minmax[(i >> 1) & 1].y,
                minmax[(i >> 2) & 1].z,
            );

            accumulator.add_point(transform.transform_point3(corner).into());
        }

        accumulator.finish().unwrap()
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
                    naive_transform_aabb(aabb, transform),
                );
            }
        }
    }

    #[test]
    fn skin_mesh() {
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.5, 0.0]],
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 3])
        .with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, vec![[0.0, 1.0, 0.0, 0.0]; 3])
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_UV_0,
            vec![[0.0, 0.00], [0.5, 0.00], [0.0, 0.25]],
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            VertexAttributeValues::Uint16x4(vec![[0, 0, 0, 0], [0, 0, 0, 0], [0, 1, 0, 0]]),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_JOINT_WEIGHT,
            vec![
                [1.00, 0.00, 0.0, 0.0],
                [1.00, 0.00, 0.0, 0.0],
                [0.75, 0.25, 0.0, 0.0],
            ],
        )
        .with_inserted_indices(Indices::U16(vec![0, 1, 2]));

        let transforms = [Affine3A::IDENTITY; 2];

        {
            let skinned = super::skin_mesh(&mesh, &transforms).unwrap();

            assert_eq!(skinned.indices(), Some(&Indices::U16(vec![0, 1, 2])));

            let attributes = super::skin_mesh(&mesh, &transforms)
                .unwrap()
                .attributes()
                .map(|(&attribute, _)| attribute)
                .collect::<Vec<_>>();

            assert_eq!(attributes.len(), 4);
            assert!(attributes.contains(&Mesh::ATTRIBUTE_POSITION));
            assert!(attributes.contains(&Mesh::ATTRIBUTE_NORMAL));
            assert!(attributes.contains(&Mesh::ATTRIBUTE_TANGENT));
            assert!(attributes.contains(&Mesh::ATTRIBUTE_UV_0));
        }

        assert!(super::skin_mesh(
            &mesh
                .clone()
                .with_removed_attribute(Mesh::ATTRIBUTE_POSITION),
            &transforms
        )
        .is_ok());

        assert!(super::skin_mesh(
            &mesh.clone().with_removed_attribute(Mesh::ATTRIBUTE_NORMAL),
            &transforms
        )
        .is_ok());

        assert!(super::skin_mesh(
            &mesh.clone().with_removed_attribute(Mesh::ATTRIBUTE_TANGENT),
            &transforms
        )
        .is_ok());

        assert!(super::skin_mesh(
            &mesh
                .clone()
                .with_removed_attribute(Mesh::ATTRIBUTE_POSITION)
                .with_removed_attribute(Mesh::ATTRIBUTE_NORMAL)
                .with_removed_attribute(Mesh::ATTRIBUTE_TANGENT),
            &transforms
        )
        .is_ok());

        assert_eq!(
            super::skin_mesh(
                &mesh
                    .clone()
                    .with_removed_attribute(Mesh::ATTRIBUTE_JOINT_INDEX),
                &transforms
            ),
            Err(SkinMeshError::MeshAttributeError(
                MeshAttributeError::MissingAttribute(Mesh::ATTRIBUTE_JOINT_INDEX.name)
            ))
        );

        assert_eq!(
            super::skin_mesh(
                &mesh
                    .clone()
                    .with_removed_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT),
                &transforms
            ),
            Err(SkinMeshError::MeshAttributeError(
                MeshAttributeError::MissingAttribute(Mesh::ATTRIBUTE_JOINT_WEIGHT.name)
            ))
        );

        assert_eq!(
            super::skin_mesh(
                &mesh.clone().with_inserted_attribute(
                    Mesh::ATTRIBUTE_JOINT_INDEX,
                    VertexAttributeValues::Uint16x4(vec![[13, 13, 13, 13]; 3]),
                ),
                &transforms
            ),
            Err(SkinMeshError::InvalidJointIndex)
        );

        assert_eq!(
            super::skin_mesh(
                &mesh
                    .clone()
                    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0, 0.0, 0.0]]),
                &transforms
            ),
            Err(SkinMeshError::MismatchedMeshAttributeLengths)
        );

        {
            let mut mesh = mesh.clone();
            mesh.asset_usage = RenderAssetUsages::RENDER_WORLD;
            drop(mesh.take_gpu_data().unwrap());

            assert_eq!(
                super::skin_mesh(&mesh, &transforms),
                Err(SkinMeshError::MeshAttributeError(
                    MeshAttributeError::MeshAccessError(MeshAccessError::ExtractedToRenderWorld)
                ))
            );
        }
    }
}
