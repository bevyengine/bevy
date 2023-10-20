use bevy_asset::{Asset, Handle};
use bevy_reflect::TypePath;
use bevy_render::{
    mesh::MeshVertexBufferLayout,
    render_asset::RenderAssets,
    render_resource::{
        AsBindGroup, AsBindGroupError, BindGroupLayout, RenderPipelineDescriptor, Shader,
        ShaderRef, SpecializedMeshPipelineError, UnpreparedBindGroup,
    },
    renderer::RenderDevice,
    texture::{FallbackImage, Image},
};

use crate::{Material, MaterialPipeline, MaterialPipelineKey, MeshPipeline, MeshPipelineKey};

pub struct MaterialExtensionPipeline {
    pub mesh_pipeline: MeshPipeline,
    pub material_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
}

pub struct MaterialExtensionKey<E: MaterialExtension> {
    pub mesh_key: MeshPipelineKey,
    pub bind_group_data: E::Data,
}

/// A subset of the `Material` trait for defining extensions to a base `Material`, such as the builtin `StandardMaterial`.
/// A user type implementing the trait should be used as the `E` generic param in an `ExtendedMaterial` struct.
pub trait MaterialExtension: Asset + AsBindGroup + Clone + Sized {
    /// Returns this material's vertex shader. If [`ShaderRef::Default`] is returned, the base material mesh vertex shader
    /// will be used.
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's fragment shader. If [`ShaderRef::Default`] is returned, the base material mesh fragment shader
    /// will be used.
    #[allow(unused_variables)]
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's prepass vertex shader. If [`ShaderRef::Default`] is returned, the base material prepass vertex shader
    /// will be used.
    fn prepass_vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's prepass fragment shader. If [`ShaderRef::Default`] is returned, the base material prepass fragment shader
    /// will be used.
    #[allow(unused_variables)]
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
    #[allow(unused_variables)]
    fn deferred_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Customizes the default [`RenderPipelineDescriptor`] for a specific entity using the entity's
    /// [`MaterialPipelineKey`] and [`MeshVertexBufferLayout`] as input.
    /// Specialization for the base material is applied before this function is called.
    #[allow(unused_variables)]
    #[inline]
    fn specialize(
        pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
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
#[derive(Asset, Clone, TypePath)]
pub struct ExtendedMaterial<B: Material, E: MaterialExtension> {
    pub base: B,
    pub extension: E,
}

impl<B: Material, E: MaterialExtension> AsBindGroup for ExtendedMaterial<B, E> {
    type Data = (<B as AsBindGroup>::Data, <E as AsBindGroup>::Data);

    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        images: &RenderAssets<Image>,
        fallback_image: &FallbackImage,
    ) -> Result<bevy_render::render_resource::UnpreparedBindGroup<Self::Data>, AsBindGroupError>
    {
        // add together the bindings of the base material and the user material
        let UnpreparedBindGroup {
            mut bindings,
            data: base_data,
        } = B::unprepared_bind_group(&self.base, layout, render_device, images, fallback_image)?;
        let extended_bindgroup = E::unprepared_bind_group(
            &self.extension,
            layout,
            render_device,
            images,
            fallback_image,
        )?;

        bindings.extend(extended_bindgroup.bindings);

        Ok(UnpreparedBindGroup {
            bindings,
            data: (base_data, extended_bindgroup.data),
        })
    }

    fn bind_group_layout_entries(
        render_device: &RenderDevice,
    ) -> Vec<bevy_render::render_resource::BindGroupLayoutEntry>
    where
        Self: Sized,
    {
        // add together the bindings of the standard material and the user material
        let mut entries = B::bind_group_layout_entries(render_device);
        entries.extend(E::bind_group_layout_entries(render_device));
        entries
    }
}

impl<B: Material, E: MaterialExtension> Material for ExtendedMaterial<B, E> {
    fn vertex_shader() -> bevy_render::render_resource::ShaderRef {
        match E::vertex_shader() {
            ShaderRef::Default => B::vertex_shader(),
            specified => specified,
        }
    }

    fn fragment_shader() -> bevy_render::render_resource::ShaderRef {
        match E::fragment_shader() {
            ShaderRef::Default => B::fragment_shader(),
            specified => specified,
        }
    }

    fn prepass_vertex_shader() -> bevy_render::render_resource::ShaderRef {
        match E::prepass_vertex_shader() {
            ShaderRef::Default => B::prepass_vertex_shader(),
            specified => specified,
        }
    }

    fn prepass_fragment_shader() -> bevy_render::render_resource::ShaderRef {
        match E::prepass_fragment_shader() {
            ShaderRef::Default => B::prepass_fragment_shader(),
            specified => specified,
        }
    }

    fn deferred_vertex_shader() -> bevy_render::render_resource::ShaderRef {
        match E::deferred_vertex_shader() {
            ShaderRef::Default => B::deferred_vertex_shader(),
            specified => specified,
        }
    }

    fn deferred_fragment_shader() -> bevy_render::render_resource::ShaderRef {
        match E::deferred_fragment_shader() {
            ShaderRef::Default => B::deferred_fragment_shader(),
            specified => specified,
        }
    }

    fn alpha_mode(&self) -> crate::AlphaMode {
        B::alpha_mode(&self.base)
    }

    fn depth_bias(&self) -> f32 {
        B::depth_bias(&self.base)
    }

    fn opaque_render_method(&self) -> crate::OpaqueRendererMethod {
        B::opaque_render_method(&self.base)
    }

    fn specialize(
        pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Call the base material's specialize function
        let MaterialPipeline::<Self> {
            mesh_pipeline,
            material_layout,
            vertex_shader,
            fragment_shader,
            ..
        } = pipeline.clone();
        let base_pipeline = MaterialPipeline::<B> {
            mesh_pipeline,
            material_layout,
            vertex_shader,
            fragment_shader,
            marker: Default::default(),
        };
        let base_key = MaterialPipelineKey::<B> {
            mesh_key: key.mesh_key,
            bind_group_data: key.bind_group_data.0,
        };
        B::specialize(&base_pipeline, descriptor, layout, base_key)?;

        // Call the extended material's specialize function afterwards
        let MaterialPipeline::<Self> {
            mesh_pipeline,
            material_layout,
            vertex_shader,
            fragment_shader,
            ..
        } = pipeline.clone();

        E::specialize(
            &MaterialExtensionPipeline {
                mesh_pipeline,
                material_layout,
                vertex_shader,
                fragment_shader,
            },
            descriptor,
            layout,
            MaterialExtensionKey {
                mesh_key: key.mesh_key,
                bind_group_data: key.bind_group_data.1,
            },
        )
    }
}
