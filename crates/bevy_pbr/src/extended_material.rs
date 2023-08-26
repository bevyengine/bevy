use bevy_reflect::{TypePath, TypeUuid, Uuid};
use bevy_render::{
    mesh::MeshVertexBufferLayout,
    render_asset::RenderAssets,
    render_resource::{
        AsBindGroup, AsBindGroupError, BindGroupLayout, RenderPipelineDescriptor,
        SpecializedMeshPipelineError, UnpreparedBindGroup,
    },
    renderer::RenderDevice,
    texture::{FallbackImage, Image},
};

use crate::{Material, MaterialPipeline, MaterialPipelineKey, StandardMaterial};

/// A material that extends the [`StandardMaterial`] with user-defined shaders
/// and data.
/// The data from both the parts will be combined and made available to the shader
/// so that the built in `pbr_fragment` function will work as expected, and custom
/// data can also be used.
/// If the material `M` returns a non-default result from `vertex_shader()` it will be used in place of the standard
/// vertex shader (`bevy_pbr::render::mesh.wgsl`). It should return the same output as the standard vertex shader.
/// If the material `M` returns a non-default result from `fragment_shader()` it will be used in place of the standard
/// fragment shader (`bevy_pbr::render::pbr_fragment.wgsl`). since all the standard material fields are
/// present, the `pbr_fragment` shader function can be called from the custom shader (see
/// the `extended_material` example).
/// Alpha mode from the extended material is ignored, only the standard material's alpha mode
/// is used.
#[derive(Clone, TypePath)]
pub struct ExtendedMaterial<M: Material> {
    pub standard: StandardMaterial,
    pub extended: M,
}

// derive uuid from (standard material uuid XOR material uuid)
impl<M: Material> TypeUuid for ExtendedMaterial<M> {
    const TYPE_UUID: bevy_reflect::Uuid =
        Uuid::from_u128(StandardMaterial::TYPE_UUID.as_u128() ^ M::TYPE_UUID.as_u128());
}

impl<M: Material> AsBindGroup for ExtendedMaterial<M> {
    type Data = (
        <StandardMaterial as AsBindGroup>::Data,
        <M as AsBindGroup>::Data,
    );

    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        images: &RenderAssets<Image>,
        fallback_image: &FallbackImage,
    ) -> Result<bevy_render::render_resource::UnpreparedBindGroup<Self::Data>, AsBindGroupError>
    {
        // add together the bindings of the standard material and the user material
        let UnpreparedBindGroup {
            mut bindings,
            data: standard_data,
        } = StandardMaterial::unprepared_bind_group(
            &self.standard,
            layout,
            render_device,
            images,
            fallback_image,
        )?;
        let extended_bindgroup = M::unprepared_bind_group(
            &self.extended,
            layout,
            render_device,
            images,
            fallback_image,
        )?;

        bindings.extend(extended_bindgroup.bindings);

        Ok(UnpreparedBindGroup {
            bindings,
            data: (standard_data, extended_bindgroup.data),
        })
    }

    fn bind_group_layout_entries(
        render_device: &RenderDevice,
    ) -> Vec<bevy_render::render_resource::BindGroupLayoutEntry>
    where
        Self: Sized,
    {
        // add together the bindings of the standard material and the user material
        let mut entries = StandardMaterial::bind_group_layout_entries(render_device);
        entries.extend(M::bind_group_layout_entries(render_device));
        entries
    }
}

impl<M: Material> Material for ExtendedMaterial<M> {
    fn vertex_shader() -> bevy_render::render_resource::ShaderRef {
        match M::vertex_shader() {
            bevy_render::render_resource::ShaderRef::Default => StandardMaterial::vertex_shader(),
            specified => specified,
        }
    }

    fn fragment_shader() -> bevy_render::render_resource::ShaderRef {
        match M::fragment_shader() {
            bevy_render::render_resource::ShaderRef::Default => StandardMaterial::fragment_shader(),
            specified => specified,
        }
    }

    fn alpha_mode(&self) -> crate::AlphaMode {
        StandardMaterial::alpha_mode(&self.standard)
    }

    fn depth_bias(&self) -> f32 {
        M::depth_bias(&self.extended) + StandardMaterial::depth_bias(&self.standard)
    }

    fn prepass_vertex_shader() -> bevy_render::render_resource::ShaderRef {
        match M::prepass_vertex_shader() {
            bevy_render::render_resource::ShaderRef::Default => {
                StandardMaterial::prepass_vertex_shader()
            }
            specified => specified,
        }
    }

    fn prepass_fragment_shader() -> bevy_render::render_resource::ShaderRef {
        match M::prepass_fragment_shader() {
            bevy_render::render_resource::ShaderRef::Default => {
                StandardMaterial::prepass_fragment_shader()
            }
            specified => specified,
        }
    }

    fn specialize(
        pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // call StandardMaterial specialize function
        let MaterialPipeline::<Self> {
            mesh_pipeline,
            material_layout,
            vertex_shader,
            fragment_shader,
            ..
        } = pipeline.clone();
        let standard_pipeline = MaterialPipeline::<StandardMaterial> {
            mesh_pipeline,
            material_layout,
            vertex_shader,
            fragment_shader,
            marker: Default::default(),
        };
        let standard_key = MaterialPipelineKey::<StandardMaterial> {
            mesh_key: key.mesh_key,
            bind_group_data: key.bind_group_data.0,
        };
        StandardMaterial::specialize(&standard_pipeline, descriptor, layout, standard_key)?;

        // call custom material specialize function afterwards
        let MaterialPipeline::<Self> {
            mesh_pipeline,
            material_layout,
            vertex_shader,
            fragment_shader,
            ..
        } = pipeline.clone();
        let m_pipeline = MaterialPipeline::<M> {
            mesh_pipeline,
            material_layout,
            vertex_shader,
            fragment_shader,
            marker: Default::default(),
        };
        let m_key = MaterialPipelineKey::<M> {
            mesh_key: key.mesh_key,
            bind_group_data: key.bind_group_data.1,
        };
        M::specialize(&m_pipeline, descriptor, layout, m_key)
    }
}
