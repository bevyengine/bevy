use bevy_reflect::{TypeUuid, Uuid};
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

#[derive(Clone)]
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
        M::alpha_mode(&self.extended)
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
