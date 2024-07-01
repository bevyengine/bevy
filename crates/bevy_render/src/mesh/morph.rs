use crate::{
    mesh::Mesh,
    render_asset::RenderAssetUsages,
    render_resource::{Extent3d, TextureDimension, TextureFormat},
    texture::Image,
};
use bevy_app::{Plugin, PostUpdate};
use bevy_asset::Handle;
use bevy_ecs::prelude::*;
use bevy_hierarchy::Children;
use bevy_math::Vec3;
use bevy_reflect::prelude::*;
use bytemuck::{Pod, Zeroable};
use std::{iter, mem};
use thiserror::Error;

const MAX_TEXTURE_WIDTH: u32 = 2048;
// NOTE: "component" refers to the element count of math objects,
// Vec3 has 3 components, Mat2 has 4 components.
const MAX_COMPONENTS: u32 = MAX_TEXTURE_WIDTH * MAX_TEXTURE_WIDTH;

/// Max target count available for [morph targets](MorphWeights).
pub const MAX_MORPH_WEIGHTS: usize = 64;

/// [Inherit weights](inherit_weights) from glTF mesh parent entity to direct
/// bevy mesh child entities (ie: glTF primitive).
pub struct MorphPlugin;
impl Plugin for MorphPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.register_type::<MorphWeights>()
            .register_type::<MeshMorphWeights>()
            .add_systems(PostUpdate, inherit_weights);
    }
}

#[derive(Error, Clone, Debug)]
pub enum MorphBuildError {
    #[error(
        "Too many vertex×components in morph target, max is {MAX_COMPONENTS}, \
        got {vertex_count}×{component_count} = {}",
        *vertex_count * *component_count as usize
    )]
    TooManyAttributes {
        vertex_count: usize,
        component_count: u32,
    },
    #[error(
        "Bevy only supports up to {} morph targets (individual poses), tried to \
        create a model with {target_count} morph targets",
        MAX_MORPH_WEIGHTS
    )]
    TooManyTargets { target_count: usize },
}

/// An image formatted for use with [`MorphWeights`] for rendering the morph target.
#[derive(Debug)]
pub struct MorphTargetImage(pub Image);

impl MorphTargetImage {
    /// Generate textures for each morph target.
    ///
    /// This accepts an "iterator of [`MorphAttributes`] iterators". Each item iterated in the top level
    /// iterator corresponds "the attributes of a specific morph target".
    ///
    /// Each pixel of the texture is a component of morph target animated
    /// attributes. So a set of 9 pixels is this morph's displacement for
    /// position, normal and tangents of a single vertex (each taking 3 pixels).
    pub fn new(
        targets: impl ExactSizeIterator<Item = impl Iterator<Item = MorphAttributes>>,
        vertex_count: usize,
        asset_usage: RenderAssetUsages,
    ) -> Result<Self, MorphBuildError> {
        let max = MAX_TEXTURE_WIDTH;
        let target_count = targets.len();
        if target_count > MAX_MORPH_WEIGHTS {
            return Err(MorphBuildError::TooManyTargets { target_count });
        }
        let component_count = (vertex_count * MorphAttributes::COMPONENT_COUNT) as u32;
        let Some((Rect(width, height), padding)) = lowest_2d(component_count, max) else {
            return Err(MorphBuildError::TooManyAttributes {
                vertex_count,
                component_count,
            });
        };
        let data = targets
            .flat_map(|mut attributes| {
                let layer_byte_count = (padding + component_count) as usize * mem::size_of::<f32>();
                let mut buffer = Vec::with_capacity(layer_byte_count);
                for _ in 0..vertex_count {
                    let Some(to_add) = attributes.next() else {
                        break;
                    };
                    buffer.extend_from_slice(bytemuck::bytes_of(&to_add));
                }
                // Pad each layer so that they fit width * height
                buffer.extend(iter::repeat(0).take(padding as usize * mem::size_of::<f32>()));
                debug_assert_eq!(buffer.len(), layer_byte_count);
                buffer
            })
            .collect();
        let extents = Extent3d {
            width,
            height,
            depth_or_array_layers: target_count as u32,
        };
        let image = Image::new(
            extents,
            TextureDimension::D3,
            data,
            TextureFormat::R32Float,
            asset_usage,
        );
        Ok(MorphTargetImage(image))
    }
}

/// Controls the [morph targets] for all child [`Handle<Mesh>`] entities. In most cases, [`MorphWeights`] should be considered
/// the "source of truth" when writing morph targets for meshes. However you can choose to write child [`MeshMorphWeights`]
/// if your situation requires more granularity. Just note that if you set [`MorphWeights`], it will overwrite child
/// [`MeshMorphWeights`] values.
///
/// This exists because Bevy's [`Mesh`] corresponds to a _single_ surface / material, whereas morph targets
/// as defined in the GLTF spec exist on "multi-primitive meshes" (where each primitive is its own surface with its own material).
/// Therefore in Bevy [`MorphWeights`] an a parent entity are the "canonical weights" from a GLTF perspective, which then
/// synchronized to child [`Handle<Mesh>`] / [`MeshMorphWeights`] (which correspond to "primitives" / "surfaces" from a GLTF perspective).
///
/// Add this to the parent of one or more [`Entities`](`Entity`) with a [`Handle<Mesh>`] with a [`MeshMorphWeights`].
///
/// [morph targets]: https://en.wikipedia.org/wiki/Morph_target_animation
#[derive(Reflect, Default, Debug, Clone, Component)]
#[reflect(Debug, Component, Default)]
pub struct MorphWeights {
    weights: Vec<f32>,
    /// The first mesh primitive assigned to these weights
    first_mesh: Option<Handle<Mesh>>,
}
impl MorphWeights {
    pub fn new(
        weights: Vec<f32>,
        first_mesh: Option<Handle<Mesh>>,
    ) -> Result<Self, MorphBuildError> {
        if weights.len() > MAX_MORPH_WEIGHTS {
            let target_count = weights.len();
            return Err(MorphBuildError::TooManyTargets { target_count });
        }
        Ok(MorphWeights {
            weights,
            first_mesh,
        })
    }
    /// The first child [`Handle<Mesh>`] primitive controlled by these weights.
    /// This can be used to look up metadata information such as [`Mesh::morph_target_names`].
    pub fn first_mesh(&self) -> Option<&Handle<Mesh>> {
        self.first_mesh.as_ref()
    }
    pub fn weights(&self) -> &[f32] {
        &self.weights
    }
    pub fn weights_mut(&mut self) -> &mut [f32] {
        &mut self.weights
    }
}

/// Control a specific [`Mesh`] instance's [morph targets]. These control the weights of
/// specific "mesh primitives" in scene formats like GLTF. They can be set manually, but
/// in most cases they should "automatically" synced by setting the [`MorphWeights`] component
/// on a parent entity.
///
/// See [`MorphWeights`] for more details on Bevy's morph target implementation.
///
/// Add this to an [`Entity`] with a [`Handle<Mesh>`] with a [`MorphAttributes`] set
/// to control individual weights of each morph target.
///
/// [morph targets]: https://en.wikipedia.org/wiki/Morph_target_animation
#[derive(Reflect, Default, Debug, Clone, Component)]
#[reflect(Debug, Component, Default)]
pub struct MeshMorphWeights {
    weights: Vec<f32>,
}
impl MeshMorphWeights {
    pub fn new(weights: Vec<f32>) -> Result<Self, MorphBuildError> {
        if weights.len() > MAX_MORPH_WEIGHTS {
            let target_count = weights.len();
            return Err(MorphBuildError::TooManyTargets { target_count });
        }
        Ok(MeshMorphWeights { weights })
    }
    pub fn weights(&self) -> &[f32] {
        &self.weights
    }
    pub fn weights_mut(&mut self) -> &mut [f32] {
        &mut self.weights
    }
}

/// Bevy meshes are gltf primitives, [`MorphWeights`] on the bevy node entity
/// should be inherited by children meshes.
///
/// Only direct children are updated, to fulfill the expectations of glTF spec.
pub fn inherit_weights(
    morph_nodes: Query<(&Children, &MorphWeights), (Without<Handle<Mesh>>, Changed<MorphWeights>)>,
    mut morph_primitives: Query<&mut MeshMorphWeights, With<Handle<Mesh>>>,
) {
    for (children, parent_weights) in &morph_nodes {
        let mut iter = morph_primitives.iter_many_mut(children);
        while let Some(mut child_weight) = iter.fetch_next() {
            child_weight.weights.clear();
            child_weight.weights.extend(&parent_weights.weights);
        }
    }
}

/// Attributes **differences** used for morph targets.
///
/// See [`MorphTargetImage`] for more information.
#[derive(Copy, Clone, PartialEq, Pod, Zeroable, Default)]
#[repr(C)]
pub struct MorphAttributes {
    /// The vertex position difference between base mesh and this target.
    pub position: Vec3,
    /// The vertex normal difference between base mesh and this target.
    pub normal: Vec3,
    /// The vertex tangent difference between base mesh and this target.
    ///
    /// Note that tangents are a `Vec4`, but only the `xyz` components are
    /// animated, as the `w` component is the sign and cannot be animated.
    pub tangent: Vec3,
}
impl From<[Vec3; 3]> for MorphAttributes {
    fn from([position, normal, tangent]: [Vec3; 3]) -> Self {
        MorphAttributes {
            position,
            normal,
            tangent,
        }
    }
}
impl MorphAttributes {
    /// How many components `MorphAttributes` has.
    ///
    /// Each `Vec3` has 3 components, we have 3 `Vec3`, for a total of 9.
    pub const COMPONENT_COUNT: usize = 9;

    pub fn new(position: Vec3, normal: Vec3, tangent: Vec3) -> Self {
        MorphAttributes {
            position,
            normal,
            tangent,
        }
    }
}

/// Integer division rounded up.
const fn div_ceil(lhf: u32, rhs: u32) -> u32 {
    (lhf + rhs - 1) / rhs
}
struct Rect(u32, u32);

/// Find the smallest rectangle of maximum edge size `max_edge` that contains
/// at least `min_includes` cells. `u32` is how many extra cells the rectangle
/// has.
///
/// The following rectangle contains 27 cells, and its longest edge is 9:
/// ```text
/// ----------------------------
/// |1 |2 |3 |4 |5 |6 |7 |8 |9 |
/// ----------------------------
/// |2 |  |  |  |  |  |  |  |  |
/// ----------------------------
/// |3 |  |  |  |  |  |  |  |  |
/// ----------------------------
/// ```
///
/// Returns `None` if `max_edge` is too small to build a rectangle
/// containing `min_includes` cells.
fn lowest_2d(min_includes: u32, max_edge: u32) -> Option<(Rect, u32)> {
    (1..=max_edge)
        .filter_map(|a| {
            let b = div_ceil(min_includes, a);
            let diff = (a * b).checked_sub(min_includes)?;
            Some((Rect(a, b), diff))
        })
        .filter_map(|(rect, diff)| (rect.1 <= max_edge).then_some((rect, diff)))
        .min_by_key(|(_, diff)| *diff)
}
