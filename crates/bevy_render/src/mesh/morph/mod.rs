mod visitors;

use bevy_app::{Plugin, PostUpdate};
use bevy_hierarchy::Children;
use thiserror::Error;

use crate::{
    render_asset::RenderAssets,
    render_resource::{Extent3d, TextureDimension, TextureFormat, TextureView},
    texture::Image,
};
use bevy_asset::Handle;
use bevy_ecs::{
    component::Component,
    prelude::ReflectComponent,
    query::{Changed, With, Without},
    system::Query,
};
use bevy_reflect::Reflect;
use std::{iter, mem};

pub use visitors::{MorphAttributes, VisitAttributes, VisitMorphTargets};

use super::Mesh;

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
pub type Result<T> = std::result::Result<T, MorphBuildError>;

/// Value of [`Mesh`]'s [morph targets]. See also [`Mesh::set_morph_targets`].
///
/// [morph targets]: https://en.wikipedia.org/wiki/Morph_target_animation
/// [`Mesh::set_morph_targets`]: super::Mesh::set_morph_targets
/// [`Mesh`]: super::Mesh
#[derive(Debug, Clone)]
pub(crate) struct MorphAttributesImage(pub(crate) Handle<Image>);
impl MorphAttributesImage {
    pub(crate) fn binding(&self, images: &RenderAssets<Image>) -> Option<TextureView> {
        Some(images.get(&self.0)?.texture_view.clone())
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

#[derive(Debug)]
pub(crate) struct MorphTargetImage {
    /// The image used with [`MorphWeights`] for rendering the morph target.
    pub(crate) image: Image,
}
impl MorphTargetImage {
    pub(crate) fn new(targets: impl VisitMorphTargets, vertex_count: u32) -> Result<Self> {
        let total_components = MorphAttributes::COMPONENT_COUNT;

        let target_count = targets.target_count();
        if target_count > MAX_MORPH_WEIGHTS {
            return Err(MorphBuildError::TooManyTargets { target_count });
        }
        let image = Self::displacements_buffer(
            targets,
            vertex_count as usize,
            target_count,
            total_components as u32,
        )?;
        Ok(MorphTargetImage { image })
    }

    /// Generate textures for each morph target.
    ///
    /// Each pixel of the texture is a component of morph target animated
    /// attributes. So a set of 9 pixels is this morph's displacement for
    /// position, normal and tangents of a single vertex (each taking 3 pixels).
    fn displacements_buffer(
        mut targets: impl VisitMorphTargets,
        vertex_count: usize,
        target_count: usize,
        total_components: u32,
    ) -> Result<Image> {
        let max = MAX_TEXTURE_WIDTH;
        let component_count = vertex_count as u32 * total_components;
        let Some((Rect(width, height), padding)) = lowest_2d(component_count, max) else {
            return Err(MorphBuildError::TooManyAttributes { vertex_count, component_count });
        };
        let data = targets
            .targets()
            .flat_map(|mut attributes| {
                let layer_byte_count = (padding + component_count) as usize * mem::size_of::<f32>();
                let mut buffer = Vec::with_capacity(layer_byte_count);
                for _ in 0..vertex_count {
                    let Some(to_add) = attributes.next_attributes() else {
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
        let image = Image::new(extents, TextureDimension::D3, data, TextureFormat::R32Float);
        Ok(image)
    }
}

/// Control a [`Mesh`]'s [morph targets].
///
/// Add this to an [`Entity`] with a [`Handle<Mesh>`] with a [`MorphAttributes`] set
/// to control individual weights of each morph target.
///
/// [morph targets]: https://en.wikipedia.org/wiki/Morph_target_animation
/// [`Entity`]: bevy_ecs::prelude::Entity
#[derive(Reflect, Default, Debug, Clone, Component)]
#[reflect(Debug, Component)]
pub struct MorphWeights {
    weights: Vec<f32>,
}
impl MorphWeights {
    pub fn new(weights: Vec<f32>) -> Result<Self> {
        if weights.len() > MAX_MORPH_WEIGHTS {
            let target_count = weights.len();
            return Err(MorphBuildError::TooManyTargets { target_count });
        }
        Ok(MorphWeights { weights })
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
    mut morph_primitives: Query<&mut MorphWeights, With<Handle<Mesh>>>,
) {
    for (children, parent_weights) in &morph_nodes {
        let mut iter = morph_primitives.iter_many_mut(children);
        while let Some(mut child_weight) = iter.fetch_next() {
            child_weight.weights.clear();
            child_weight.weights.extend(&parent_weights.weights);
        }
    }
}

/// [Inherit weights](inherit_weights) from glTF mesh parent entity to direct
/// bevy mesh child entities (ie: glTF primitive).
pub struct MorphPlugin;
impl Plugin for MorphPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(PostUpdate, inherit_weights);
    }
}
