use bevy_asset::{AsAssetId, Asset, AssetId, Handle};
use bevy_ecs::{component::Component, entity::Entity, prelude::ReflectComponent, system::Query};
use bevy_math::{bounding::Aabb3d, Affine3A, Mat4, Vec3, Vec3A};
use bevy_reflect::prelude::*;
use bevy_transform::components::GlobalTransform;
use core::ops::Deref;

use crate::{Mesh, VertexAttributeValues};

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

// XXX TODO: Document why this is different from `Aabb3d`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct JointBounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl From<JointBounds> for Aabb3d {
    fn from(value: JointBounds) -> Self {
        Self {
            min: value.min.into(),
            max: value.max.into(),
        }
    }
}

impl From<Aabb3d> for JointBounds {
    fn from(value: Aabb3d) -> Self {
        Self {
            min: value.min.into(),
            max: value.max.into(),
        }
    }
}

// XXX TODO: Consider folding `bounds` and `bounds_index_to_joint_index` into
// one array.
#[derive(Clone, Debug, PartialEq)]
pub struct SkinnedMeshBounds {
    // Model-space bounds of each skinned joint.
    pub bounds: Box<[JointBounds]>,

    // Mapping from `SkinnedMeshBounds::bounds` index to `SkinnedMesh::joints` index.
    pub bounds_index_to_joint_index: Box<[JointIndex]>,
}

impl SkinnedMeshBounds {
    pub fn iter(&self) -> impl Iterator<Item = (&JointBounds, &JointIndex)> {
        self.bounds
            .iter()
            .zip(self.bounds_index_to_joint_index.iter())
    }
}

// XXX TODO: Avoid dependency on `Mesh`? Take attributes instead.
pub(crate) fn create_skinned_mesh_bounds(mesh: &Mesh) -> Option<SkinnedMeshBounds> {
    let max_joint_index = InfluenceIterator::new(mesh)
        .map(|i| i.joint_index)
        .reduce(Ord::max)?;

    // XXX TODO: Maybe change this to use `AabbAccumulator`.

    let mut optional_bounds: Box<[Option<Aabb3d>]> =
        vec![None; (max_joint_index as usize) + 1].into();

    for influence in InfluenceIterator::new(mesh) {
        extend(
            &mut optional_bounds[influence.joint_index as usize],
            Vec3A::from(influence.position),
        );
    }

    let num_bounds = optional_bounds.iter().filter(|o| o.is_some()).count();

    if num_bounds == 0 {
        // XXX TODO: Should this be an error?
        return None;
    }

    let mut bounds = Vec::<JointBounds>::with_capacity(num_bounds);
    let mut bounds_index_to_joint_index = Vec::<JointIndex>::with_capacity(num_bounds);

    for (joint_index, joint_optional_bounds) in optional_bounds.iter().enumerate() {
        if let &Some(joint_bounds) = joint_optional_bounds {
            bounds.push(joint_bounds.into());
            bounds_index_to_joint_index.push(joint_index as JointIndex);
        };
    }

    assert!(bounds.len() == num_bounds);
    assert!(bounds_index_to_joint_index.len() == num_bounds);

    Some(SkinnedMeshBounds {
        bounds: bounds.into(),
        bounds_index_to_joint_index: bounds_index_to_joint_index.into(),
    })
}

pub fn entity_aabb_from_skinned_mesh_bounds(
    joint_entities: &Query<&GlobalTransform>,
    mesh: &Mesh,
    skinned_mesh: &SkinnedMesh,
    skinned_mesh_inverse_bindposes: &SkinnedMeshInverseBindposes,
    world_from_entity: &GlobalTransform,
) -> Option<Aabb3d> {
    let Some(skinned_mesh_bounds) = mesh.skinned_mesh_bounds() else {
        return None;
    };

    let mut worldspace_entity_aabb_accumulator = AabbAccumulator::new();

    for (&joint_bounds, &joint_index) in skinned_mesh_bounds.iter() {
        let Some(&joint_entity) = skinned_mesh.joints.get(joint_index as usize) else {
            // XXX TODO: Error?
            continue;
        };

        let Ok(&world_from_joint) = joint_entities.get(joint_entity) else {
            continue;
        };

        let Some(joint_from_model) = skinned_mesh_inverse_bindposes
            .get(joint_index as usize)
            .map(|&m| Affine3A::from_mat4(m))
        else {
            // XXX TODO: Error?
            continue;
        };

        let world_from_model = world_from_joint.affine() * joint_from_model;

        let worldspace_joint_aabb = transform_bounds(joint_bounds, world_from_model);

        worldspace_entity_aabb_accumulator.add(worldspace_joint_aabb);
    }

    let worldspace_entity_aabb = worldspace_entity_aabb_accumulator.finish()?;

    let entityspace_entity_aabb = transform_bounds(
        worldspace_entity_aabb.into(),
        world_from_entity.affine().inverse(),
    );

    Some(entityspace_entity_aabb)
}

// Match the `Mesh` limits on joint indices (ATTRIBUTE_JOINT_INDEX = VertexFormat::Uint16x4)
//
// XXX TODO: Where should this go?
pub type JointIndex = u16;

// XXX TODO: Where should this go?
pub const MAX_INFLUENCES: usize = 4;

// Return an AABB that contains the transformed joint bounds.
//
// Algorithm from "Transforming Axis-Aligned Bounding Boxes", James Arvo, Graphics Gems (1990).
#[inline]
pub fn transform_bounds(input: JointBounds, transform: Affine3A) -> Aabb3d {
    let rs = transform.matrix3;
    let t = transform.translation;

    let e_x = rs.x_axis * Vec3A::splat(input.min.x);
    let e_y = rs.y_axis * Vec3A::splat(input.min.y);
    let e_z = rs.z_axis * Vec3A::splat(input.min.z);

    let f_x = rs.x_axis * Vec3A::splat(input.max.x);
    let f_y = rs.y_axis * Vec3A::splat(input.max.y);
    let f_z = rs.z_axis * Vec3A::splat(input.max.z);

    let min_x = e_x.min(f_x);
    let min_y = e_y.min(f_y);
    let min_z = e_z.min(f_z);

    let max_x = e_x.max(f_x);
    let max_y = e_y.max(f_y);
    let max_z = e_z.max(f_z);

    let min = t + min_x + min_y + min_z;
    let max = t + max_x + max_y + max_z;

    Aabb3d { min, max }
}

struct AabbAccumulator {
    min: Vec3A,
    max: Vec3A,
}

impl AabbAccumulator {
    fn new() -> Self {
        Self {
            min: Vec3A::MAX,
            max: Vec3A::MIN,
        }
    }

    fn add(&mut self, aabb: Aabb3d) {
        self.min = self.min.min(aabb.min);
        self.max = self.max.max(aabb.max);
    }

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

// Return `optional_aabb` extended to include `point`. If `optional_aabb` is
// none, return the AABB of `point`.
fn extend(optional_aabb: &mut Option<Aabb3d>, point: Vec3A) {
    match *optional_aabb {
        Some(aabb) => {
            *optional_aabb = Some(Aabb3d {
                min: point.min(aabb.min),
                max: point.max(aabb.max),
            });
        }
        None => {
            *optional_aabb = Some(Aabb3d {
                min: point,
                max: point,
            });
        }
    }
}

struct Influence {
    position: Vec3,
    joint_index: JointIndex,
}

/// Iterator over all vertex influences with non-zero weight.
#[derive(Default)]
struct InfluenceIterator<'a> {
    vertex_index: usize,
    influence_index: usize,
    positions: &'a [[f32; 3]],
    joint_indices: &'a [[u16; 4]],
    joint_weights: &'a [[f32; 4]],
}

impl<'a> InfluenceIterator<'a> {
    // XXX TODO: Avoid dependency on `Mesh`? Take attributes instead.
    fn new(mesh: &'a Mesh) -> Self {
        if let (
            Some(VertexAttributeValues::Float32x3(positions)),
            Some(VertexAttributeValues::Uint16x4(joint_indices)),
            Some(VertexAttributeValues::Float32x4(joint_weights)),
        ) = (
            mesh.attribute(Mesh::ATTRIBUTE_POSITION),
            mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX),
            mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT),
        ) {
            if (joint_indices.len() != positions.len()) | (joint_weights.len() != positions.len()) {
                // TODO: Should be an error?
                return InfluenceIterator::default();
            }

            return InfluenceIterator {
                vertex_index: 0,
                influence_index: 0,
                positions,
                joint_indices,
                joint_weights,
            };
        }

        InfluenceIterator::default()
    }
}

impl Iterator for InfluenceIterator<'_> {
    type Item = Influence;

    fn next(&mut self) -> Option<Influence> {
        loop {
            assert!(self.influence_index <= MAX_INFLUENCES);
            assert!(self.vertex_index <= self.positions.len());

            if self.influence_index >= MAX_INFLUENCES {
                self.influence_index = 0;
                self.vertex_index += 1;
            }

            if self.vertex_index >= self.positions.len() {
                break None;
            }

            let position = Vec3::from_array(self.positions[self.vertex_index]);
            let joint_index = self.joint_indices[self.vertex_index][self.influence_index];
            let joint_weight = self.joint_weights[self.vertex_index][self.influence_index];

            self.influence_index += 1;

            if joint_weight > 0.0 {
                break Some(Influence {
                    position,
                    joint_index,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_math::{bounding::BoundingVolume, vec3a};

    #[test]
    fn aabb_accumulator() {
        let aabb_0 = Aabb3d {
            min: vec3a(1.0, 2.0, 3.0),
            max: vec3a(5.0, 4.0, 3.0),
        };

        let aabb_1 = Aabb3d {
            min: vec3a(-99.0, 2.0, 3.0),
            max: vec3a(5.0, 4.0, 3.0),
        };

        let aabb_2 = Aabb3d {
            min: vec3a(1.0, 2.0, 3.0),
            max: vec3a(5.0, 99.0, 3.0),
        };

        {
            let none = AabbAccumulator::new();

            assert_eq!(none.finish(), None);
        }

        {
            let mut one = AabbAccumulator::new();
            one.add(aabb_0);

            assert_eq!(one.finish(), Some(aabb_0));
        }

        {
            let mut multiple = AabbAccumulator::new();
            multiple.add(aabb_0);
            multiple.add(aabb_1);
            multiple.add(aabb_2);

            let expected = aabb_0.merge(&aabb_1.merge(&aabb_2));

            assert_eq!(multiple.finish(), Some(expected));
        }
    }
}
