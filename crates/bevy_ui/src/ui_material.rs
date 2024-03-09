use std::hash::Hash;

use bevy_asset::Asset;
use bevy_render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderRef};

/// Materials are used alongside [`UiMaterialPlugin`](crate::UiMaterialPlugin) and [`MaterialNodeBundle`](crate::prelude::MaterialNodeBundle)
/// to spawn entities that are rendered with a specific [`UiMaterial`] type. They serve as an easy to use high level
/// way to render `Node` entities with custom shader logic.
///
/// `UiMaterials` must implement [`AsBindGroup`] to define how data will be transferred to the GPU and bound in shaders.
/// [`AsBindGroup`] can be derived, which makes generating bindings straightforward. See the [`AsBindGroup`] docs for details.
///
/// Materials must also implement [`Asset`] so they can be treated as such.
///
/// If you are only using the fragment shader, make sure your shader imports the `UiVertexOutput`
/// from `bevy_ui::ui_vertex_output` and uses it as the input of your fragment shader like the
/// example below does.
///
/// # Example
///
/// Here is a simple [`UiMaterial`] implementation. The [`AsBindGroup`] derive has many features. To see what else is available,
/// check out the [`AsBindGroup`] documentation.
/// ```
/// # use bevy_ui::prelude::*;
/// # use bevy_ecs::prelude::*;
/// # use bevy_reflect::TypePath;
/// # use bevy_render::{render_resource::{AsBindGroup, ShaderRef}, texture::Image};
/// # use bevy_color::LinearRgba;
/// # use bevy_asset::{Handle, AssetServer, Assets, Asset};
///
/// #[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
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
/// // All functions on `UiMaterial` have default impls. You only need to implement the
/// // functions that are relevant for your material.
/// impl UiMaterial for CustomMaterial {
///     fn fragment_shader() -> ShaderRef {
///         "shaders/custom_material.wgsl".into()
///     }
/// }
///
/// // Spawn an entity using `CustomMaterial`.
/// fn setup(mut commands: Commands, mut materials: ResMut<Assets<CustomMaterial>>, asset_server: Res<AssetServer>) {
///     commands.spawn(MaterialNodeBundle {
///         style: Style {
///             width: Val::Percent(100.0),
///             ..Default::default()
///         },
///         material: materials.add(CustomMaterial {
///             color: LinearRgba::RED,
///             color_texture: asset_server.load("some_image.png"),
///         }),
///         ..Default::default()
///     });
/// }
/// ```
/// In WGSL shaders, the material's binding would look like this:
///
/// If you only use the fragment shader make sure to import `UiVertexOutput` from
/// `bevy_ui::ui_vertex_output` in your wgsl shader.
/// Also note that bind group 0 is always bound to the [`View Uniform`](bevy_render::view::ViewUniform)
/// and the [`Globals Uniform`](bevy_render::globals::GlobalsUniform).
///
/// ```wgsl
/// #import bevy_ui::ui_vertex_output UiVertexOutput
///
/// struct CustomMaterial {
///     color: vec4<f32>,
/// }
///
/// @group(1) @binding(0)
/// var<uniform> material: CustomMaterial;
/// @group(1) @binding(1)
/// var color_texture: texture_2d<f32>;
/// @group(1) @binding(2)
/// var color_sampler: sampler;
///
/// @fragment
/// fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
///
/// }
/// ```
pub trait UiMaterial: AsBindGroup + Asset + Clone + Sized {
    /// Returns this materials vertex shader. If [`ShaderRef::Default`] is returned, the default UI
    /// vertex shader will be used.
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this materials fragment shader. If [`ShaderRef::Default`] is returned, the default
    /// UI fragment shader will be used.
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    #[allow(unused_variables)]
    #[inline]
    fn specialize(descriptor: &mut RenderPipelineDescriptor, key: UiMaterialKey<Self>) {}
}

pub struct UiMaterialKey<M: UiMaterial> {
    pub hdr: bool,
    pub bind_group_data: M::Data,
}

impl<M: UiMaterial> Eq for UiMaterialKey<M> where M::Data: PartialEq {}

impl<M: UiMaterial> PartialEq for UiMaterialKey<M>
where
    M::Data: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.hdr == other.hdr && self.bind_group_data == other.bind_group_data
    }
}

impl<M: UiMaterial> Clone for UiMaterialKey<M>
where
    M::Data: Clone,
{
    fn clone(&self) -> Self {
        Self {
            hdr: self.hdr,
            bind_group_data: self.bind_group_data.clone(),
        }
    }
}

impl<M: UiMaterial> Hash for UiMaterialKey<M>
where
    M::Data: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hdr.hash(state);
        self.bind_group_data.hash(state);
    }
}
