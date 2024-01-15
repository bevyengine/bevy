use std::hash::Hash;

use bevy_asset::Asset;
use bevy_ecs::component::Component;
use bevy_render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderRef};

use crate::SpritePipelineKey;

/// The `SpriteMaterial` trait is implemented by materials that are used alongside
/// [`SpriteMaterialPlugin`](crate::SpriteMaterialPlugin) and
/// [`SpriteWithMaterialBundle`](crate::prelude::SpriteWithMaterialBundle) to render sprites
/// with custom shader logic.
///
/// `SpriteMaterials` must implement [`AsBindGroup`] to define how data will be transferred to
/// the GPU and bound in shaders. [`AsBindGroup`] can be derived, which makes generating bindings
/// straightforward. See the [`AsBindGroup`] docs for details.
///
/// Materials must also implement [`Asset`] so they can be treated as assets and loaded by the
/// [`AssetServer`].
///
/// If you are only using the fragment shader, make sure your shader imports the `SpriteVertexOutput`
/// from `bevy_sprite::sprite_vertex_output` and uses it as the input of your fragment shader like the
/// example below does.
///
/// ### Example
///
/// Here is a simple [`SpriteMaterial`] implementation. The [`AsBindGroup`] derive has many features.
/// To see what else is available, check out the [`AsBindGroup`] documentation.
///
/// ```rust
/// //Displays a single [`Sprite`], created from an image, and applies a grayscale effect to it.
///
/// use bevy::prelude::*;
/// use bevy_internal::{
///     render::render_resource::{AsBindGroup, ShaderRef},
///     sprite::{SpriteMaterial, SpriteMaterialPlugin, SpriteWithMaterialBundle},
/// };
///
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         // Add the grayscale material plugin to the app
///         .add_plugins(SpriteMaterialPlugin::<GrayScale>::default())
///         .add_systems(Startup, setup)
///         .run();
/// }
///
/// fn setup(
///     mut commands: Commands,
///     asset_server: Res<AssetServer>,
///     mut sprite_materials: ResMut<Assets<GrayScale>>,
/// ) {
///     commands.spawn(Camera2dBundle::default());
///
///     // Create a sprite with a grayscale material
///     commands.spawn(SpriteWithMaterialBundle {
///         texture: asset_server.load("textures/rpg/chars/sensei/sensei.png"),
///         material: sprite_materials.add(GrayScale {}),
///         ..default()
///     });
/// }
///
/// #[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
/// struct GrayScale {}
///
/// impl SpriteMaterial for GrayScale {
///     fn fragment_shader() -> ShaderRef {
///         // Return the shader reference for the grayscale fragment shader
///         "shaders/grayscale.wgsl".into()
///     }
/// }
/// ```
///
/// In WGSL shaders, the material's binding would look like this:
///
/// ```wgsl
/// // Bind the sprite texture and sampler to the first binding in the first group
/// @group(1) @binding(0) var sprite_texture: texture_2d<f32>;
/// @group(1) @binding(1) var sprite_sampler: sampler;
///
/// // Fragment shader entry point
/// @fragment
/// fn fragment(in: SpriteVertexOutput) -> @location(0) vec4<f32> {
///     // Calculate the color of the fragment by multiplying the input color with the sampled texture color
///     var color = in.color * textureSample(sprite_texture, sprite_sampler, in.uv);
///
///     // Convert the color to grayscale using the formula:
///     // gray = 0.21 * red + 0.72 * green + 0.07 * blue
///     let g = 0.21 * color.r + 0.72 * color.g + 0.07 * color.b;
///
///     // Return the grayscale color with the same alpha value as the input color
///     return vec4<f32>(g, g, g, color.a);
/// }
/// ```
///
/// Note that bind group 0 is always bound to the [`View Uniform`](bevy_render::view::ViewUniform)
/// and the [`Globals Uniform`](bevy_render::globals::GlobalsUniform).
///
/// The `SpriteMaterial` trait has two associated functions, `vertex_shader` and `fragment_shader`,
/// which return the vertex and fragment shaders to be used for the material. If
/// [`ShaderRef::Default`] is returned, the default sprite vertex shader and sprite material
/// fragment shader will be used, respectively.
///
/// The `specialize` function can be implemented to customize the render pipeline descriptor for
/// specific materials.
pub trait SpriteMaterial: AsBindGroup + Asset + Clone + Sized {
    /// Returns this materials vertex shader. If [`ShaderRef::Default`] is returned, the default UI
    /// vertex shader will be used.
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this materials fragment shader. If [`ShaderRef::Default`] is returned, the default
    /// SpriteMaterial fragment shader will be used.
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    #[allow(unused_variables)]
    #[inline]
    fn specialize(descriptor: &mut RenderPipelineDescriptor, key: SpriteMaterialKey<Self>) {}
}

pub struct SpriteMaterialKey<M: SpriteMaterial> {
    pub pipeline_key: SpritePipelineKey,
    pub bind_group_data: M::Data,
}

impl<M: SpriteMaterial> Eq for SpriteMaterialKey<M> where M::Data: PartialEq {}

impl<M: SpriteMaterial> PartialEq for SpriteMaterialKey<M>
where
    M::Data: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.pipeline_key == other.pipeline_key && self.bind_group_data == other.bind_group_data
    }
}

impl<M: SpriteMaterial> Clone for SpriteMaterialKey<M>
where
    M::Data: Clone,
{
    fn clone(&self) -> Self {
        Self {
            pipeline_key: self.pipeline_key.clone(),
            bind_group_data: self.bind_group_data.clone(),
        }
    }
}

impl<M: SpriteMaterial> Hash for SpriteMaterialKey<M>
where
    M::Data: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.pipeline_key.hash(state);
        self.bind_group_data.hash(state);
    }
}

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct SpriteMaterialMarke;
