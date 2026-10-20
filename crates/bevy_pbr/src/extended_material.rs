use bevy_asset::Asset;
use bevy_ecs::system::SystemParamItem;
use bevy_material::{AlphaMode, OpaqueRendererMethod};
use bevy_mesh::MeshVertexBufferLayoutRef;
use bevy_reflect::{impl_type_path, Reflect};
use bevy_render::{
    combined_bind_group as cbg,
    render_resource::{
        AsBindGroup, AsBindGroupError, BindGroupBuilder, BindGroupLayout, BindGroupLayoutEntry,
        BindlessDescriptor, BindlessSlabResourceLimit, RenderPipelineDescriptor,
        SpecializedMeshPipelineError,
    },
    renderer::RenderDevice,
};
use bevy_shader::ShaderRef;

use crate::{Material, MaterialPipeline, MaterialPipelineKey, MeshPipeline, MeshPipelineKey};

pub struct MaterialExtensionPipeline {
    pub mesh_pipeline: MeshPipeline,
}

pub struct MaterialExtensionKey<E: MaterialExtension> {
    pub mesh_key: MeshPipelineKey,
    pub bind_group_data: E::Data,
}

/// A subset of the `Material` trait for defining extensions to a base `Material`, such as the builtin `StandardMaterial`.
///
/// A user type implementing the trait should be used as the `E` generic param in an `ExtendedMaterial` struct.
pub trait MaterialExtension: Asset + AsBindGroup + Clone + Sized {
    /// Returns this material's vertex shader. If [`ShaderRef::Default`] is returned, the base material mesh vertex shader
    /// will be used.
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's fragment shader. If [`ShaderRef::Default`] is returned, the base material mesh fragment shader
    /// will be used.
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    // Returns this material’s AlphaMode. If None is returned, the base material alpha mode will be used.
    fn alpha_mode() -> Option<AlphaMode> {
        None
    }

    /// Controls if the prepass is enabled for the Material.
    /// For more information about what a prepass is, see the [`bevy_core_pipeline::prepass`] docs.
    #[inline]
    fn enable_prepass() -> bool {
        true
    }

    /// Controls if shadows are enabled for the Material.
    #[inline]
    fn enable_shadows() -> bool {
        true
    }

    /// Controls whether order independent transparency is enabled for the Material.
    /// This is ignored if the camera does not have [`bevy_core_pipeline::oit::OrderIndependentTransparencySettings`]
    /// or if [`Self::alpha_mode`] is not supported by OIT.
    #[inline]
    fn enable_oit() -> bool {
        true
    }

    /// Returns this material's prepass vertex shader. If [`ShaderRef::Default`] is returned, the base material prepass vertex shader
    /// will be used.
    fn prepass_vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's prepass fragment shader. If [`ShaderRef::Default`] is returned, the base material prepass fragment shader
    /// will be used.
    fn prepass_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's deferred vertex shader. If [`ShaderRef::Default`] is returned, the base material deferred vertex shader
    /// will be used.
    fn deferred_vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's prepass fragment shader. If [`ShaderRef::Default`] is returned, the base material deferred fragment shader
    /// will be used.
    fn deferred_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's [`crate::meshlet::MeshletMesh`] fragment shader. If [`ShaderRef::Default`] is returned,
    /// the default meshlet mesh fragment shader will be used.
    #[cfg(feature = "meshlet")]
    fn meshlet_mesh_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's [`crate::meshlet::MeshletMesh`] prepass fragment shader. If [`ShaderRef::Default`] is returned,
    /// the default meshlet mesh prepass fragment shader will be used.
    #[cfg(feature = "meshlet")]
    fn meshlet_mesh_prepass_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's [`crate::meshlet::MeshletMesh`] deferred fragment shader. If [`ShaderRef::Default`] is returned,
    /// the default meshlet mesh deferred fragment shader will be used.
    #[cfg(feature = "meshlet")]
    fn meshlet_mesh_deferred_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Customizes the default [`RenderPipelineDescriptor`] for a specific entity using the entity's
    /// [`MaterialPipelineKey`] and [`MeshVertexBufferLayoutRef`] as input.
    /// Specialization for the base material is applied before this function is called.
    #[expect(
        unused_variables,
        reason = "The parameters here are intentionally unused by the default implementation; however, putting underscores here will result in the underscores being copied by rust-analyzer's tab completion."
    )]
    #[inline]
    fn specialize(
        pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        Ok(())
    }
}

/// A material that extends a base [`Material`] with additional shaders and data.
///
/// The data from both materials will be combined and made available to the shader
/// so that shader functions built for the base material (and referencing the base material
/// bindings) will work as expected, and custom alterations based on custom data can also be used.
///
/// If the extension `E` returns a non-default result from `vertex_shader()` it will be used in place of the base
/// material's vertex shader.
///
/// If the extension `E` returns a non-default result from `fragment_shader()` it will be used in place of the base
/// fragment shader.
///
/// When used with `StandardMaterial` as the base, all the standard material fields are
/// present, so the `pbr_fragment` shader functions can be called from the extension shader (see
/// the `extended_material` example).
#[derive(Asset, Clone, Debug, Reflect)]
#[reflect(type_path = false)]
#[reflect(Clone)]
pub struct ExtendedMaterial<B: Material, E: MaterialExtension> {
    pub base: B,
    pub extension: E,
}

impl<B, E> Default for ExtendedMaterial<B, E>
where
    B: Material + Default,
    E: MaterialExtension + Default,
{
    fn default() -> Self {
        Self {
            base: B::default(),
            extension: E::default(),
        }
    }
}

// We don't use the `TypePath` derive here due to a bug where `#[reflect(type_path = false)]`
// causes the `TypePath` derive to not generate an implementation.
impl_type_path!((in bevy_pbr::extended_material) ExtendedMaterial<B: Material, E: MaterialExtension>);

impl<B: Material, E: MaterialExtension> AsBindGroup for ExtendedMaterial<B, E> {
    type Data = cbg::CombinedBindGroupData<B::Data, E::Data>;
    type Param = (B::Param, E::Param);

    fn bindless_slot_count() -> Option<BindlessSlabResourceLimit> {
        cbg::bindless_slot_count::<B, E>()
    }

    fn bindless_supported(render_device: &RenderDevice) -> bool {
        B::bindless_supported(render_device) && E::bindless_supported(render_device)
    }

    fn label() -> &'static str {
        E::label()
    }

    fn bind_group_data(&self) -> Self::Data {
        cbg::bind_group_data(&self.base, &self.extension)
    }

    fn build_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        param: &mut SystemParamItem<'_, '_, Self::Param>,
        force_no_bindless: bool,
        output: &mut BindGroupBuilder,
    ) -> Result<(), AsBindGroupError> {
        cbg::build_bind_group(
            &self.base,
            &self.extension,
            layout,
            render_device,
            param,
            force_no_bindless,
            output,
        )
    }

    fn bind_group_layout_entries(
        render_device: &RenderDevice,
        force_no_bindless: bool,
    ) -> Vec<BindGroupLayoutEntry>
    where
        Self: Sized,
    {
        cbg::bind_group_layout_entries::<B, E>(render_device, force_no_bindless)
    }

    fn bindless_descriptor() -> Option<BindlessDescriptor> {
        cbg::bindless_descriptor::<B, E>()
    }
}

impl<B: Material, E: MaterialExtension> Material for ExtendedMaterial<B, E> {
    fn vertex_shader() -> ShaderRef {
        match E::vertex_shader() {
            ShaderRef::Default => B::vertex_shader(),
            specified => specified,
        }
    }

    fn fragment_shader() -> ShaderRef {
        match E::fragment_shader() {
            ShaderRef::Default => B::fragment_shader(),
            specified => specified,
        }
    }

    fn alpha_mode(&self) -> AlphaMode {
        match E::alpha_mode() {
            Some(specified) => specified,
            None => B::alpha_mode(&self.base),
        }
    }

    fn opaque_render_method(&self) -> OpaqueRendererMethod {
        B::opaque_render_method(&self.base)
    }

    fn depth_bias(&self) -> f32 {
        B::depth_bias(&self.base)
    }

    fn reads_view_transmission_texture(&self) -> bool {
        B::reads_view_transmission_texture(&self.base)
    }

    fn enable_prepass() -> bool {
        E::enable_prepass()
    }

    fn enable_shadows() -> bool {
        E::enable_shadows()
    }

    fn enable_oit() -> bool {
        E::enable_oit()
    }

    fn prepass_vertex_shader() -> ShaderRef {
        match E::prepass_vertex_shader() {
            ShaderRef::Default => B::prepass_vertex_shader(),
            specified => specified,
        }
    }

    fn prepass_fragment_shader() -> ShaderRef {
        match E::prepass_fragment_shader() {
            ShaderRef::Default => B::prepass_fragment_shader(),
            specified => specified,
        }
    }

    fn deferred_vertex_shader() -> ShaderRef {
        match E::deferred_vertex_shader() {
            ShaderRef::Default => B::deferred_vertex_shader(),
            specified => specified,
        }
    }

    fn deferred_fragment_shader() -> ShaderRef {
        match E::deferred_fragment_shader() {
            ShaderRef::Default => B::deferred_fragment_shader(),
            specified => specified,
        }
    }

    #[cfg(feature = "meshlet")]
    fn meshlet_mesh_fragment_shader() -> ShaderRef {
        match E::meshlet_mesh_fragment_shader() {
            ShaderRef::Default => B::meshlet_mesh_fragment_shader(),
            specified => specified,
        }
    }

    #[cfg(feature = "meshlet")]
    fn meshlet_mesh_prepass_fragment_shader() -> ShaderRef {
        match E::meshlet_mesh_prepass_fragment_shader() {
            ShaderRef::Default => B::meshlet_mesh_prepass_fragment_shader(),
            specified => specified,
        }
    }

    #[cfg(feature = "meshlet")]
    fn meshlet_mesh_deferred_fragment_shader() -> ShaderRef {
        match E::meshlet_mesh_deferred_fragment_shader() {
            ShaderRef::Default => B::meshlet_mesh_deferred_fragment_shader(),
            specified => specified,
        }
    }

    fn specialize(
        pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Call the base material's specialize function
        let base_key = MaterialPipelineKey::<B> {
            mesh_key: key.mesh_key,
            bind_group_data: key.bind_group_data.base,
        };
        B::specialize(pipeline, descriptor, layout, base_key)?;

        // Call the extended material's specialize function afterwards
        E::specialize(
            &MaterialExtensionPipeline {
                mesh_pipeline: pipeline.mesh_pipeline.clone(),
            },
            descriptor,
            layout,
            MaterialExtensionKey {
                mesh_key: key.mesh_key,
                bind_group_data: key.bind_group_data.extension,
            },
        )
    }
}

#[deprecated = "Use `bevy_render::combined_bind_group::CombinedBindGroupData` instead"]
pub type MaterialExtensionBindGroupData<B, E> = cbg::CombinedBindGroupData<B, E>;
