use super::Mesh;
use bevy_asset::Handle;
use bevy_ecs::prelude::*;
use bevy_math::Vec3;
use bevy_reflect::prelude::*;
use bytemuck::{Pod, Zeroable};
use encase::ShaderType;
use thiserror::Error;

/// The maximum size of the morph target texture, if morph target textures are
/// in use on the current platform.
pub const MAX_TEXTURE_WIDTH: u32 = 2048;

/// Max target count available for [morph targets](MorphWeights).
pub const MAX_MORPH_WEIGHTS: usize = 256;

/// The maximum number of morph target components, if morph target textures are
/// in use on the current platform.
///
/// NOTE: "component" refers to the element count of math objects,
/// Vec3 has 3 components, Mat2 has 4 components.
const MAX_COMPONENTS: u32 = MAX_TEXTURE_WIDTH * MAX_TEXTURE_WIDTH;

#[derive(Error, Clone, Debug)]
pub enum MorphBuildError {
    #[error(
        "Too many vertex components in morph target, max is {MAX_COMPONENTS}, \
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

/// Controls the [morph targets] for all child [`Mesh3d`](crate::Mesh3d)
/// entities. In most cases, [`MorphWeights`] should be considered the "source
/// of truth" when writing [morph targets] for meshes. However you can choose to
/// write child [`MeshMorphWeights`] if your situation requires more
/// granularity. Just note that if you set [`MorphWeights`], it will overwrite
/// child [`MeshMorphWeights`] values.
///
/// `MorphWeights` works together with the [`MeshMorphWeights`] component. When
/// a `MeshMorphWeights` is set to `MeshMorphWeights::Reference`, it references
/// another entity that is expected to contain a `MorphWeights` component. This
/// allows multiple meshes to share a single `MorphWeights` component.
///
/// ```
/// # use bevy_asset::prelude::*;
/// # use bevy_ecs::prelude::*;
/// # use bevy_mesh::Mesh;
/// # use bevy_mesh::morph::*;
/// # #[derive(Component)]
/// # struct Mesh3d(Handle<Mesh>);
/// fn setup(mut commands: Commands, mesh_handle: Handle<Mesh>) {
///     // Create the `MorphWeights` component.
///     let weights_component = MorphWeights::new(vec![0.0, 0.5, 1.0], None).unwrap();
///
///     // Spawn an entity that contains the `MorphWeights` component.
///     let weights_entity = commands.spawn(weights_component).id();
///
///     // Spawn another entity with a mesh and a `MeshMorphWeights` component
///     // that references `weights_entity`.
///     let mesh_entity = commands.spawn((
///         Mesh3d(mesh_handle.clone()),
///         MeshMorphWeights::Reference(weights_entity),
///     ));
/// }
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
    /// The first child [`Mesh3d`](crate::Mesh3d) primitive controlled by these weights.
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

/// A component that controls the [morph targets] of a mesh. Must be assigned
/// to an entity with a [`Mesh3d`](crate::Mesh3d) component.
///
/// [morph targets]: https://en.wikipedia.org/wiki/Morph_target_animation
#[derive(Reflect, Debug, Clone, Component)]
#[reflect(Debug, Component, Clone)]
pub enum MeshMorphWeights {
    Value {
        weights: Vec<f32>,
    },
    /// A reference to an entity containing a [`MorphWeights`] component. This
    /// allows a single `MorphWeights` component to control the morph targets
    /// of multiple meshes.
    ///
    /// See [`MorphWeights`] for an example.
    Reference(#[entities] Entity),
}

/// Attributes **differences** used for morph targets.
#[derive(Copy, Clone, PartialEq, Debug, Reflect, ShaderType, Pod, Zeroable, Default)]
#[reflect(Clone, Default)]
#[repr(C)]
pub struct MorphAttributes {
    /// The vertex position difference between base mesh and this target.
    pub position: Vec3,
    /// Padding to ensure that vectors start on 16-byte boundaries.
    pub pad_a: f32,
    /// The vertex normal difference between base mesh and this target.
    pub normal: Vec3,
    /// Padding to ensure that vectors start on 16-byte boundaries.
    pub pad_b: f32,
    /// The vertex tangent difference between base mesh and this target.
    ///
    /// Note that tangents are a `Vec4`, but only the `xyz` components are
    /// animated, as the `w` component is the sign and cannot be animated.
    pub tangent: Vec3,
    /// Padding to ensure that vectors start on 16-byte boundaries.
    pub pad_c: f32,
}

impl From<[Vec3; 3]> for MorphAttributes {
    fn from([position, normal, tangent]: [Vec3; 3]) -> Self {
        MorphAttributes {
            position,
            normal,
            tangent,
            pad_a: 0.0,
            pad_b: 0.0,
            pad_c: 0.0,
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
            pad_a: 0.0,
            pad_b: 0.0,
            pad_c: 0.0,
        }
    }
}
