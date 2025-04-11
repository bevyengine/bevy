//! Provides functionality for creating 2d materials

mod commands;
pub mod key;
mod pipeline;
pub mod plugin;
mod render;

use bevy_asset::{AsAssetId, Asset, AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_render::{
    mesh::MeshVertexBufferLayoutRef,
    render_resource::{
        AsBindGroup, RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError,
    },
};

use derive_more::derive::From;
use key::Material2dKey;

/// Materials are used alongside [`Material2dPlugin`](plugin::Material2dPlugin),
/// [`Mesh2d`](bevy_render::mesh::Mesh2d), and [`MeshMaterial2d`]
/// to spawn entities that are rendered with a specific [`Material2d`] type. They serve as an easy to use high level
/// way to render [`Mesh2d`](bevy_render::mesh::Mesh2d) entities with custom shader logic.
///
/// Materials must implement [`AsBindGroup`] to define how data will be transferred to the GPU and bound in shaders.
/// [`AsBindGroup`] can be derived, which makes generating bindings straightforward. See the [`AsBindGroup`] docs for details.
///
/// # Example
///
/// Here is a simple [`Material2d`] implementation. The [`AsBindGroup`] derive has many features. To see what else is available,
/// check out the [`AsBindGroup`] documentation.
///
/// ```
/// # use bevy_render_2d::material::{Material2d, MeshMaterial2d};
/// # use bevy_ecs::prelude::*;
/// # use bevy_image::Image;
/// # use bevy_reflect::TypePath;
/// # use bevy_render::{mesh::{Mesh, Mesh2d}, render_resource::{AsBindGroup, ShaderRef}};
/// # use bevy_color::{LinearRgba, palettes::basic::RED};
/// # use bevy_asset::{Handle, AssetServer, Assets, Asset};
/// # use bevy_math::primitives::Circle;
/// #
/// #[derive(AsBindGroup, Debug, Clone, Asset, TypePath)]
/// pub struct CustomMaterial {
///     // Uniform bindings must implement `ShaderType`, which will be used to convert the value to
///     // its shader-compatible equivalent. Most core math types already implement `ShaderType`.
///     #[uniform(0)]
///     color: LinearRgba,
///     // Images can be bound as textures in shaders. If the Image's sampler is also needed, just
///     // add the sampler attribute with a different binding index.
///     #[texture(1)]
///     #[sampler(2)]
///     color_texture: Handle<Image>,
/// }
///
/// // All functions on `Material2d` have default impls. You only need to implement the
/// // functions that are relevant for your material.
/// impl Material2d for CustomMaterial {
///     fn fragment_shader() -> ShaderRef {
///         "shaders/custom_material.wgsl".into()
///     }
/// }
///
/// // Spawn an entity with a mesh using `CustomMaterial`.
/// fn setup(
///     mut commands: Commands,
///     mut meshes: ResMut<Assets<Mesh>>,
///     mut materials: ResMut<Assets<CustomMaterial>>,
///     asset_server: Res<AssetServer>,
/// ) {
///     commands.spawn((
///         Mesh2d(meshes.add(Circle::new(50.0))),
///         MeshMaterial2d(materials.add(CustomMaterial {
///             color: RED.into(),
///             color_texture: asset_server.load("some_image.png"),
///         })),
///     ));
/// }
/// ```
///
/// In WGSL shaders, the material's binding would look like this:
///
/// ```wgsl
/// struct CustomMaterial {
///     color: vec4<f32>,
/// }
///
/// @group(2) @binding(0) var<uniform> material: CustomMaterial;
/// @group(2) @binding(1) var color_texture: texture_2d<f32>;
/// @group(2) @binding(2) var color_sampler: sampler;
/// ```
pub trait Material2d: AsBindGroup + Asset + Clone + Sized {
    /// Returns this material's vertex shader. If [`ShaderRef::Default`] is returned, the default mesh vertex shader
    /// will be used.
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's fragment shader. If [`ShaderRef::Default`] is returned, the default mesh fragment shader
    /// will be used.
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Add a bias to the view depth of the mesh which can be used to force a specific render order.
    #[inline]
    fn depth_bias(&self) -> f32 {
        0.0
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Opaque
    }

    /// Customizes the default [`RenderPipelineDescriptor`].
    #[expect(
        unused_variables,
        reason = "The parameters here are intentionally unused by the default implementation; however, putting underscores here will result in the underscores being copied by rust-analyzer's tab completion."
    )]
    #[inline]
    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        Ok(())
    }
}

/// A [material](Material2d) used for rendering a [`Mesh2d`](bevy_render::mesh::Mesh2d).
///
/// See [`Material2d`] for general information about 2D materials and how to implement your own materials.
///
/// # Example
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_render::{mesh::{Mesh, Mesh2d}, render_resource::AsBindGroup};
/// # use bevy_render_2d::material::{MeshMaterial2d, Material2d, AlphaMode2d};
/// # use bevy_color::{Color, palettes::basic::RED};
/// # use bevy_asset::{Asset, Assets};
/// # use bevy_math::primitives::Circle;
/// # use bevy_reflect::Reflect;
/// #
/// # // Defining locally so that there is no need to depend on `bevy_sprite`
/// # #[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
/// # struct ColorMaterial {}
/// # impl ColorMaterial {
/// #   pub fn from_color(_color: impl Into<Color>) -> Self { Self {} }
/// # }
/// # impl Material2d for ColorMaterial {
/// #     fn alpha_mode(&self) -> AlphaMode2d {
/// #         AlphaMode2d::Opaque
/// #     }
/// # }
/// // Spawn an entity with a mesh using `ColorMaterial`.
/// fn setup(
///     mut commands: Commands,
///     mut meshes: ResMut<Assets<Mesh>>,
///     mut materials: ResMut<Assets<ColorMaterial>>,
/// ) {
///     commands.spawn((
///         Mesh2d(meshes.add(Circle::new(50.0))),
///         MeshMaterial2d(materials.add(ColorMaterial::from_color(RED))),
///     ));
/// }
/// ```
///
/// [`MeshMaterial2d`]: crate::material::MeshMaterial2d
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect, From)]
#[reflect(Component, Default, Clone)]
pub struct MeshMaterial2d<M: Material2d>(pub Handle<M>);

impl<M: Material2d> Default for MeshMaterial2d<M> {
    fn default() -> Self {
        Self(Handle::default())
    }
}

impl<M: Material2d> PartialEq for MeshMaterial2d<M> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<M: Material2d> Eq for MeshMaterial2d<M> {}

impl<M: Material2d> From<MeshMaterial2d<M>> for AssetId<M> {
    fn from(material: MeshMaterial2d<M>) -> Self {
        material.id()
    }
}

impl<M: Material2d> From<&MeshMaterial2d<M>> for AssetId<M> {
    fn from(material: &MeshMaterial2d<M>) -> Self {
        material.id()
    }
}

impl<M: Material2d> AsAssetId for MeshMaterial2d<M> {
    type Asset = M;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.id()
    }
}

/// Sets how a 2d material's base color alpha channel is used for transparency.
/// Currently, this only works with [`Mesh2d`](bevy_render::mesh::Mesh2d). Sprites are always transparent.
///
/// This is very similar to [`AlphaMode`](bevy_render::alpha::AlphaMode) but this only applies to 2d meshes.
/// We use a separate type because 2d doesn't support all the transparency modes that 3d does.
#[derive(Debug, Default, Reflect, Copy, Clone, PartialEq)]
#[reflect(Default, Debug, Clone)]
pub enum AlphaMode2d {
    /// Base color alpha values are overridden to be fully opaque (1.0).
    #[default]
    Opaque,
    /// Reduce transparency to fully opaque or fully transparent
    /// based on a threshold.
    ///
    /// Compares the base color alpha value to the specified threshold.
    /// If the value is below the threshold,
    /// considers the color to be fully transparent (alpha is set to 0.0).
    /// If it is equal to or above the threshold,
    /// considers the color to be fully opaque (alpha is set to 1.0).
    Mask(f32),
    /// The base color alpha value defines the opacity of the color.
    /// Standard alpha-blending is used to blend the fragment's color
    /// with the color behind it.
    Blend,
}
