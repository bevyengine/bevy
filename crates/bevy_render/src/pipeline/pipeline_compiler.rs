use super::{
    state_descriptors::PrimitiveTopology, IndexFormat, PipelineDescriptor, VertexBufferDescriptors,
};
use crate::{
    renderer::RenderResourceContext,
    shader::{Shader, ShaderSource},
};
use bevy_asset::{Assets, Handle};
use bevy_property::{Properties, Property};
use bevy_utils::{HashMap, HashSet};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, Debug, Properties)]
pub struct PipelineSpecialization {
    pub shader_specialization: ShaderSpecialization,
    pub primitive_topology: PrimitiveTopology,
    pub dynamic_bindings: Vec<DynamicBinding>,
    pub index_format: IndexFormat,
    pub sample_count: u32,
}

impl Default for PipelineSpecialization {
    fn default() -> Self {
        Self {
            sample_count: 1,
            shader_specialization: Default::default(),
            primitive_topology: Default::default(),
            dynamic_bindings: Default::default(),
            index_format: IndexFormat::Uint32,
        }
    }
}

impl PipelineSpecialization {
    pub fn empty() -> &'static PipelineSpecialization {
        pub static EMPTY: Lazy<PipelineSpecialization> = Lazy::new(PipelineSpecialization::default);
        &EMPTY
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Default, Property, Serialize, Deserialize)]
pub struct ShaderSpecialization {
    pub shader_defs: HashSet<String>,
}

#[derive(Debug)]
struct SpecializedShader {
    shader: Handle<Shader>,
    specialization: ShaderSpecialization,
}

#[derive(Debug)]
struct SpecializedPipeline {
    pipeline: Handle<PipelineDescriptor>,
    specialization: PipelineSpecialization,
}

#[derive(Clone, Eq, PartialEq, Debug, Default, Serialize, Deserialize, Property)]
pub struct DynamicBinding {
    pub bind_group: u32,
    pub binding: u32,
}

#[derive(Debug, Default)]
pub struct PipelineCompiler {
    specialized_shaders: HashMap<Handle<Shader>, Vec<SpecializedShader>>,
    specialized_pipelines: HashMap<Handle<PipelineDescriptor>, Vec<SpecializedPipeline>>,
}

impl PipelineCompiler {
    fn compile_shader(
        &mut self,
        shaders: &mut Assets<Shader>,
        shader_handle: &Handle<Shader>,
        shader_specialization: &ShaderSpecialization,
    ) -> Handle<Shader> {
        let specialized_shaders = self
            .specialized_shaders
            .entry(*shader_handle)
            .or_insert_with(Vec::new);

        let shader = shaders.get(shader_handle).unwrap();

        // don't produce new shader if the input source is already spirv
        if let ShaderSource::Spirv(_) = shader.source {
            return *shader_handle;
        }

        if let Some(specialized_shader) =
            specialized_shaders
                .iter()
                .find(|current_specialized_shader| {
                    current_specialized_shader.specialization == *shader_specialization
                })
        {
            // if shader has already been compiled with current configuration, use existing shader
            specialized_shader.shader
        } else {
            // if no shader exists with the current configuration, create new shader and compile
            let shader_def_vec = shader_specialization
                .shader_defs
                .iter()
                .cloned()
                .collect::<Vec<String>>();
            let compiled_shader = shader.get_spirv_shader(Some(&shader_def_vec));
            let specialized_handle = shaders.add(compiled_shader);
            specialized_shaders.push(SpecializedShader {
                shader: specialized_handle,
                specialization: shader_specialization.clone(),
            });
            specialized_handle
        }
    }

    pub fn get_specialized_pipeline(
        &self,
        pipeline: Handle<PipelineDescriptor>,
        specialization: &PipelineSpecialization,
    ) -> Option<Handle<PipelineDescriptor>> {
        self.specialized_pipelines
            .get(&pipeline)
            .and_then(|specialized_pipelines| {
                specialized_pipelines
                    .iter()
                    .find(|current_specialized_pipeline| {
                        &current_specialized_pipeline.specialization == specialization
                    })
            })
            .map(|specialized_pipeline| specialized_pipeline.pipeline)
    }

    pub fn compile_pipeline(
        &mut self,
        render_resource_context: &dyn RenderResourceContext,
        pipelines: &mut Assets<PipelineDescriptor>,
        shaders: &mut Assets<Shader>,
        source_pipeline: Handle<PipelineDescriptor>,
        vertex_buffer_descriptors: &VertexBufferDescriptors,
        pipeline_specialization: &PipelineSpecialization,
    ) -> Handle<PipelineDescriptor> {
        let source_descriptor = pipelines.get(&source_pipeline).unwrap();
        let mut specialized_descriptor = source_descriptor.clone();
        specialized_descriptor.shader_stages.vertex = self.compile_shader(
            shaders,
            &specialized_descriptor.shader_stages.vertex,
            &pipeline_specialization.shader_specialization,
        );
        specialized_descriptor.shader_stages.fragment = specialized_descriptor
            .shader_stages
            .fragment
            .as_ref()
            .map(|fragment| {
                self.compile_shader(
                    shaders,
                    fragment,
                    &pipeline_specialization.shader_specialization,
                )
            });

        specialized_descriptor.reflect_layout(
            shaders,
            true,
            Some(vertex_buffer_descriptors),
            &pipeline_specialization.dynamic_bindings,
        );

        specialized_descriptor.sample_count = pipeline_specialization.sample_count;
        specialized_descriptor.primitive_topology = pipeline_specialization.primitive_topology;
        specialized_descriptor.index_format = pipeline_specialization.index_format;

        let specialized_pipeline_handle = pipelines.add(specialized_descriptor);
        render_resource_context.create_render_pipeline(
            specialized_pipeline_handle,
            pipelines.get(&specialized_pipeline_handle).unwrap(),
            &shaders,
        );

        let specialized_pipelines = self
            .specialized_pipelines
            .entry(source_pipeline)
            .or_insert_with(Vec::new);
        specialized_pipelines.push(SpecializedPipeline {
            pipeline: specialized_pipeline_handle,
            specialization: pipeline_specialization.clone(),
        });

        specialized_pipeline_handle
    }

    pub fn iter_compiled_pipelines(
        &self,
        pipeline_handle: Handle<PipelineDescriptor>,
    ) -> Option<impl Iterator<Item = &Handle<PipelineDescriptor>>> {
        if let Some(compiled_pipelines) = self.specialized_pipelines.get(&pipeline_handle) {
            Some(
                compiled_pipelines
                    .iter()
                    .map(|specialized_pipeline| &specialized_pipeline.pipeline),
            )
        } else {
            None
        }
    }

    pub fn iter_all_compiled_pipelines(&self) -> impl Iterator<Item = &Handle<PipelineDescriptor>> {
        self.specialized_pipelines
            .values()
            .map(|compiled_pipelines| {
                compiled_pipelines
                    .iter()
                    .map(|specialized_pipeline| &specialized_pipeline.pipeline)
            })
            .flatten()
    }
}
