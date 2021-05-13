use super::{ComputePipelineDescriptor, ShaderSpecialization, SpecializedShader};
use crate::{
    pipeline::BindType,
    renderer::RenderResourceContext,
    shader::{Shader, ShaderError},
};
use bevy_asset::{Assets, Handle};
use bevy_reflect::Reflect;
use bevy_utils::{HashMap, HashSet};
use once_cell::sync::Lazy;

#[derive(Clone, Eq, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
pub struct ComputePipelineSpecialization {
    pub shader_specialization: ShaderSpecialization,
    pub dynamic_bindings: HashSet<String>,
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

#[derive(Debug)]
struct ComputeSpecializedPipeline {
    pipeline: Handle<ComputePipelineDescriptor>,
    specialization: ComputePipelineSpecialization,
}

#[derive(Debug, Default)]
pub struct ComputePipelineCompiler {
    specialized_shaders: HashMap<Handle<Shader>, Vec<SpecializedShader>>,
    specialized_shader_pipelines: HashMap<Handle<Shader>, Vec<Handle<ComputePipelineDescriptor>>>,
    specialized_pipelines:
        HashMap<Handle<ComputePipelineDescriptor>, Vec<ComputeSpecializedPipeline>>,
}

impl ComputePipelineCompiler {
    fn compile_shader(
        &mut self,
        render_resource_context: &dyn RenderResourceContext,
        shaders: &mut Assets<Shader>,
        shader_handle: &Handle<Shader>,
        shader_specialization: &ShaderSpecialization,
    ) -> Result<Handle<Shader>, ShaderError> {
        let specialized_shaders = self
            .specialized_shaders
            .entry(shader_handle.clone_weak())
            .or_insert_with(Vec::new);

        let shader = shaders.get(shader_handle).unwrap();

        if let Some(specialized_shader) =
            specialized_shaders
                .iter()
                .find(|current_specialized_shader| {
                    current_specialized_shader.specialization == *shader_specialization
                })
        {
            // if shader has already been compiled with current configuration, use existing shader
            Ok(specialized_shader.shader.clone_weak())
        } else {
            // if no shader exists with the current configuration, create new shader and compile
            let shader_def_vec = shader_specialization
                .shader_defs
                .iter()
                .cloned()
                .collect::<Vec<String>>();
            let compiled_shader =
                render_resource_context.get_specialized_shader(shader, Some(&shader_def_vec))?;
            let specialized_handle = shaders.add(compiled_shader);
            let weak_specialized_handle = specialized_handle.clone_weak();
            specialized_shaders.push(SpecializedShader {
                shader: specialized_handle,
                specialization: shader_specialization.clone(),
            });
            Ok(weak_specialized_handle)
        }
    }

    pub fn get_specialized_pipeline(
        &self,
        pipeline: &Handle<ComputePipelineDescriptor>,
        specialization: &ComputePipelineSpecialization,
    ) -> Option<Handle<ComputePipelineDescriptor>> {
        self.specialized_pipelines
            .get(pipeline)
            .and_then(|specialized_pipelines| {
                specialized_pipelines
                    .iter()
                    .find(|current_specialized_pipeline| {
                        &current_specialized_pipeline.specialization == specialization
                    })
            })
            .map(|specialized_pipeline| specialized_pipeline.pipeline.clone_weak())
    }

    pub fn compile_pipeline(
        &mut self,
        render_resource_context: &dyn RenderResourceContext,
        pipelines: &mut Assets<ComputePipelineDescriptor>,
        shaders: &mut Assets<Shader>,
        source_pipeline: &Handle<ComputePipelineDescriptor>,
        pipeline_specialization: &ComputePipelineSpecialization,
    ) -> Handle<ComputePipelineDescriptor> {
        let source_descriptor = pipelines.get(source_pipeline).unwrap();
        let mut specialized_descriptor = source_descriptor.clone();
        let specialized_compute_shader = self
            .compile_shader(
                render_resource_context,
                shaders,
                &specialized_descriptor.shader_stages.compute,
                &pipeline_specialization.shader_specialization,
            )
            .unwrap_or_else(|e| panic_shader_error(e));
        specialized_descriptor.shader_stages.compute = specialized_compute_shader.clone_weak();

        let mut layout = render_resource_context.reflect_compute_pipeline_layout(
            &shaders,
            &specialized_descriptor.shader_stages,
            true,
        );

        if !pipeline_specialization.dynamic_bindings.is_empty() {
            // set binding uniforms to dynamic if render resource bindings use dynamic
            for bind_group in layout.bind_groups.iter_mut() {
                let mut binding_changed = false;
                for binding in bind_group.bindings.iter_mut() {
                    if pipeline_specialization
                        .dynamic_bindings
                        .iter()
                        .any(|b| b == &binding.name)
                    {
                        if let BindType::Uniform {
                            ref mut has_dynamic_offset,
                            ..
                        } = binding.bind_type
                        {
                            *has_dynamic_offset = true;
                            binding_changed = true;
                        }
                    }
                }

                if binding_changed {
                    bind_group.update_id();
                }
            }
        }
        specialized_descriptor.layout = Some(layout);

        let specialized_pipeline_handle = pipelines.add(specialized_descriptor);
        render_resource_context.create_compute_pipeline(
            specialized_pipeline_handle.clone_weak(),
            pipelines.get(&specialized_pipeline_handle).unwrap(),
            &shaders,
        );

        // track specialized shader pipelines
        self.specialized_shader_pipelines
            .entry(specialized_compute_shader)
            .or_insert_with(Default::default)
            .push(source_pipeline.clone_weak());

        let specialized_pipelines = self
            .specialized_pipelines
            .entry(source_pipeline.clone_weak())
            .or_insert_with(Vec::new);
        let weak_specialized_pipeline_handle = specialized_pipeline_handle.clone_weak();
        specialized_pipelines.push(ComputeSpecializedPipeline {
            pipeline: specialized_pipeline_handle,
            specialization: pipeline_specialization.clone(),
        });

        weak_specialized_pipeline_handle
    }

    pub fn iter_compiled_pipelines(
        &self,
        pipeline_handle: Handle<ComputePipelineDescriptor>,
    ) -> Option<impl Iterator<Item = &Handle<ComputePipelineDescriptor>>> {
        self.specialized_pipelines
            .get(&pipeline_handle)
            .map(|compiled_pipelines| {
                compiled_pipelines
                    .iter()
                    .map(|specialized_pipeline| &specialized_pipeline.pipeline)
            })
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

    /// Update specialized shaders and remove any related specialized
    /// pipelines and assets.
    pub fn update_shader(
        &mut self,
        shader: &Handle<Shader>,
        pipelines: &mut Assets<ComputePipelineDescriptor>,
        shaders: &mut Assets<Shader>,
        render_resource_context: &dyn RenderResourceContext,
    ) -> Result<(), ShaderError> {
        if let Some(specialized_shaders) = self.specialized_shaders.get_mut(shader) {
            for specialized_shader in specialized_shaders {
                // Recompile specialized shader. If it fails, we bail immediately.
                let shader_def_vec = specialized_shader
                    .specialization
                    .shader_defs
                    .iter()
                    .cloned()
                    .collect::<Vec<String>>();
                let new_handle =
                    shaders.add(render_resource_context.get_specialized_shader(
                        shaders.get(shader).unwrap(),
                        Some(&shader_def_vec),
                    )?);

                // Replace handle and remove old from assets.
                let old_handle = std::mem::replace(&mut specialized_shader.shader, new_handle);
                shaders.remove(&old_handle);

                // Find source pipelines that use the old specialized
                // shader, and remove from tracking.
                if let Some(source_pipelines) =
                    self.specialized_shader_pipelines.remove(&old_handle)
                {
                    // Remove all specialized pipelines from tracking
                    // and asset storage. They will be rebuilt on next
                    // draw.
                    for source_pipeline in source_pipelines {
                        if let Some(specialized_pipelines) =
                            self.specialized_pipelines.remove(&source_pipeline)
                        {
                            for p in specialized_pipelines {
                                pipelines.remove(p.pipeline);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

fn panic_shader_error(error: ShaderError) -> ! {
    let msg = error.to_string();
    let msg = msg
        .trim_end()
        .trim_end_matches("Debug log:") // if this matches, then there wasn't a debug log anyways
        .trim_end();
    panic!("{}\n", msg);
}
