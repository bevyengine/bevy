use super::Mesh;
use bevy_asset::{Handle, RenderAssetUsages};
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_math::Vec3;
use bevy_reflect::prelude::*;
use bytemuck::{Pod, Zeroable};
use thiserror::Error;
use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

const MAX_TEXTURE_WIDTH: u32 = 2048;
// NOTE: "component" refers to the element count of math objects,
// Vec3 has 3 components, Mat2 has 4 components.
const MAX_COMPONENTS: u32 = MAX_TEXTURE_WIDTH * MAX_TEXTURE_WIDTH;

/// Max target count available for [morph targets](MorphWeights).
pub const MAX_MORPH_WEIGHTS: usize = 64;

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
                let layer_byte_count = (padding + component_count) as usize * size_of::<f32>();
                let mut buffer = Vec::with_capacity(layer_byte_count);
                for _ in 0..vertex_count {
                    let Some(to_add) = attributes.next() else {
                        break;
                    };
                    buffer.extend_from_slice(bytemuck::bytes_of(&to_add));
                }
                // Pad each layer so that they fit width * height
                buffer.extend(core::iter::repeat_n(0, padding as usize * size_of::<f32>()));
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

/// Controls the [morph targets] of one or more meshes.
///
/// Any entity can contain a `MorphWeights` component. An entity with a `Mesh3d`
/// component can then reference it by adding a [`MeshMorphWeights`]
/// component.
///
/// Here, a single `MorphTargets` component is used to drive multiple meshes:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_asset::prelude::*;
/// # use bevy_mesh::*;
/// # use bevy_mesh::morph::*;
/// # #[derive(Component)]
/// # struct Mesh3d(Handle<Mesh>);
/// fn setup(mut commands: Commands, meshes: &[Handle<Mesh>]) {
///     // Create the `MorphWeights` component.
///     let weights_component = MorphWeights::new(
///         vec![0.0, 0.5, 1.0],
///         Some(meshes[0].clone())
///     ).unwrap();
///
///     // Spawn an entity that contains the weights. Or we could have added them to
///     // an existing entity.
///     let weights_entity = commands.spawn(weights_component).id();
///
///     for mesh in meshes {
///         // The `MeshMorphWeights` component references the entity with `MorphWeights`.
///         commands.spawn((
///             MeshMorphWeights(weights_entity),
///             Mesh3d(mesh.clone())
///         ));
///     }
/// }
/// ```
///
/// Any changes to the `MorphWeights` component are automatically copied to the
/// mesh.
///
/// In the simplest case, a `MorphTargets` component and a mesh can be in one
/// entity. The `MeshMorphTargets` component is still required.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_asset::prelude::*;
/// # use bevy_mesh::*;
/// # use bevy_mesh::morph::*;
/// # #[derive(Component)]
/// # struct Mesh3d(Handle<Mesh>);
/// # fn setup(mut commands: Commands, mesh: Handle<Mesh>) {
/// # let weights_component = MorphWeights::new(vec![0.0, 0.5, 1.0], Some(mesh.clone())).unwrap();
/// let entity = commands.spawn(weights_component).id();
///
/// commands.entity(entity).insert(
///     (MeshMorphWeights(entity),
///     Mesh3d(mesh.clone())
/// ));
/// # }
/// ```
///
/// [morph targets]: https://en.wikipedia.org/wiki/Morph_target_animation
#[derive(Reflect, Default, Debug, Clone, Component)]
#[reflect(Debug, Component, Default, Clone)]
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
    /// The first child `Mesh3d` primitive controlled by these weights.
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
    pub fn clear_weights(&mut self) {
        self.weights.clear();
    }
    pub fn extend_weights(&mut self, weights: &[f32]) {
        self.weights.extend(weights);
    }
}

/// Controls the [morph targets] of a specific `Mesh3d` by referencing an
/// entity with a [`MorphWeights`] component. Multiple meshes can reference a
/// single `MorphWeights` component.
///
/// [morph targets]: https://en.wikipedia.org/wiki/Morph_target_animation
#[derive(Reflect, Debug, Clone, Component)]
#[reflect(Debug, Component, Clone)]
pub struct MeshMorphWeights(#[entities] pub Entity);

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
            let b = min_includes.div_ceil(a);
            let diff = (a * b).checked_sub(min_includes)?;
            Some((Rect(a, b), diff))
        })
        .filter_map(|(rect, diff)| (rect.1 <= max_edge).then_some((rect, diff)))
        .min_by_key(|(_, diff)| *diff)
}
