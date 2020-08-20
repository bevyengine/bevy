use super::{ComputePipelineDescriptor, DynamicBinding, ShaderSpecialization, SpecializedShader};
use crate::{
    renderer::RenderResourceContext,
    shader::{Shader, ShaderSource},
};
use bevy_asset::{Assets, Handle};
use bevy_property::Properties;
use once_cell::sync::Lazy;
use std::collections::HashMap;

#[derive(Clone, Eq, PartialEq, Debug, Properties)]
pub struct ComputePipelineSpecialization {
    pub shader_specialization: ShaderSpecialization,
    pub dynamic_bindings: Vec<DynamicBinding>,
}

impl Default for ComputePipelineSpecialization {
    fn default() -> Self {
        Self {
            shader_specialization: Default::default(),
            dynamic_bindings: Default::default(),
        }
    }
}

impl ComputePipelineSpecialization {
    pub fn empty() -> &'static ComputePipelineSpecialization {
        pub static EMPTY: Lazy<ComputePipelineSpecialization> =
            Lazy::new(ComputePipelineSpecialization::default);
        &EMPTY
    }
}

struct ComputeSpecializedPipeline {
    pipeline: Handle<ComputePipelineDescriptor>,
    specialization: ComputePipelineSpecialization,
}

#[derive(Default)]
pub struct ComputePipelineCompiler {
    specialized_shaders: HashMap<Handle<Shader>, Vec<SpecializedShader>>,
    specialized_pipelines:
        HashMap<Handle<ComputePipelineDescriptor>, Vec<ComputeSpecializedPipeline>>,
}

impl ComputePipelineCompiler {
    // TODO: Share some of this with PipelineCompiler.
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
        pipeline: Handle<ComputePipelineDescriptor>,
        specialization: &ComputePipelineSpecialization,
    ) -> Option<Handle<ComputePipelineDescriptor>> {
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
        pipelines: &mut Assets<ComputePipelineDescriptor>,
        shaders: &mut Assets<Shader>,
        source_pipeline: Handle<ComputePipelineDescriptor>,
        pipeline_specialization: &ComputePipelineSpecialization,
    ) -> Handle<ComputePipelineDescriptor> {
        let source_descriptor = pipelines.get(&source_pipeline).unwrap();
        let mut specialized_descriptor = source_descriptor.clone();
        specialized_descriptor.shader_stages.compute = self.compile_shader(
            shaders,
            &specialized_descriptor.shader_stages.compute,
            &pipeline_specialization.shader_specialization,
        );

        specialized_descriptor.reflect_layout(shaders, &pipeline_specialization.dynamic_bindings);

        let specialized_pipeline_handle = pipelines.add(specialized_descriptor);
        render_resource_context.create_compute_pipeline(
            specialized_pipeline_handle,
            pipelines.get(&specialized_pipeline_handle).unwrap(),
            &shaders,
        );

        let specialized_pipelines = self
            .specialized_pipelines
            .entry(source_pipeline)
            .or_insert_with(Vec::new);
        specialized_pipelines.push(ComputeSpecializedPipeline {
            pipeline: specialized_pipeline_handle,
            specialization: pipeline_specialization.clone(),
        });

        specialized_pipeline_handle
    }

    pub fn iter_compiled_pipelines(
        &self,
        pipeline_handle: Handle<ComputePipelineDescriptor>,
    ) -> Option<impl Iterator<Item = &Handle<ComputePipelineDescriptor>>> {
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

    pub fn iter_all_compiled_pipelines(
        &self,
    ) -> impl Iterator<Item = &Handle<ComputePipelineDescriptor>> {
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
