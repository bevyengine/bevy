use super::{state_descriptors::PrimitiveTopology, IndexFormat, PipelineDescriptor};
use crate::{
    pipeline::{
        BindType, InputStepMode, VertexAttributeDescriptor, VertexBufferDescriptor, VertexFormat,
        VERTEX_FALLBACK_LAYOUT_NAME,
    },
    renderer::RenderResourceContext,
    shader::{Shader, ShaderSource},
};
use bevy_asset::{Assets, Handle};
use bevy_property::{Properties, Property};
use bevy_utils::{HashMap, HashSet};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Clone, Eq, PartialEq, Debug, Properties)]
pub struct PipelineSpecialization {
    pub shader_specialization: ShaderSpecialization,
    pub primitive_topology: PrimitiveTopology,
    pub dynamic_bindings: Vec<String>,
    pub index_format: IndexFormat,
    pub vertex_buffer_descriptor: VertexBufferDescriptor,
    pub sample_count: u32,
}

impl Default for PipelineSpecialization {
    fn default() -> Self {
        Self {
            sample_count: 1,
            index_format: IndexFormat::Uint32,
            shader_specialization: Default::default(),
            primitive_topology: Default::default(),
            dynamic_bindings: Default::default(),
            vertex_buffer_descriptor: Default::default(),
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
            .entry(shader_handle.clone_weak())
            .or_insert_with(Vec::new);

        let shader = shaders.get(shader_handle).unwrap();

        // don't produce new shader if the input source is already spirv
        if let ShaderSource::Spirv(_) = shader.source {
            return shader_handle.clone_weak();
        }

        if let Some(specialized_shader) =
            specialized_shaders
                .iter()
                .find(|current_specialized_shader| {
                    current_specialized_shader.specialization == *shader_specialization
                })
        {
            // if shader has already been compiled with current configuration, use existing shader
            specialized_shader.shader.clone_weak()
        } else {
            // if no shader exists with the current configuration, create new shader and compile
            let shader_def_vec = shader_specialization
                .shader_defs
                .iter()
                .cloned()
                .collect::<Vec<String>>();
            let compiled_shader = shader.get_spirv_shader(Some(&shader_def_vec));
            let specialized_handle = shaders.add(compiled_shader);
            let weak_specialized_handle = specialized_handle.clone_weak();
            specialized_shaders.push(SpecializedShader {
                shader: specialized_handle,
                specialization: shader_specialization.clone(),
            });
            weak_specialized_handle
        }
    }

    pub fn get_specialized_pipeline(
        &self,
        pipeline: &Handle<PipelineDescriptor>,
        specialization: &PipelineSpecialization,
    ) -> Option<Handle<PipelineDescriptor>> {
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
        pipelines: &mut Assets<PipelineDescriptor>,
        shaders: &mut Assets<Shader>,
        source_pipeline: &Handle<PipelineDescriptor>,
        pipeline_specialization: &PipelineSpecialization,
    ) -> Handle<PipelineDescriptor> {
        let source_descriptor = pipelines.get(source_pipeline).unwrap();
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

        let mut layout = render_resource_context.reflect_pipeline_layout(
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
                            ref mut dynamic, ..
                        } = binding.bind_type
                        {
                            *dynamic = true;
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

        // create a vertex layout that provides all attributes from either the specialized vertex buffers or a zero buffer
        let mut pipeline_layout = specialized_descriptor.layout.as_mut().unwrap();
        // the vertex buffer descriptor of the mesh
        let mesh_vertex_buffer_descriptor = &pipeline_specialization.vertex_buffer_descriptor;

        // the vertex buffer descriptor that will be used for this pipeline
        let mut compiled_vertex_buffer_descriptor = VertexBufferDescriptor {
            step_mode: InputStepMode::Vertex,
            stride: mesh_vertex_buffer_descriptor.stride,
            ..Default::default()
        };

        let mut fallback_vertex_buffer_descriptor = VertexBufferDescriptor {
            name: Cow::Borrowed(VERTEX_FALLBACK_LAYOUT_NAME),
            stride: VertexFormat::Float4.get_size(), //TODO: use smallest possible format
            ..Default::default()
        };
        for shader_vertex_attribute in pipeline_layout.vertex_buffer_descriptors.iter() {
            let shader_vertex_attribute = shader_vertex_attribute
                .attributes
                .get(0)
                .expect("Reflected layout has no attributes.");

            if let Some(target_vertex_attribute) = mesh_vertex_buffer_descriptor
                .attributes
                .iter()
                .find(|x| x.name == shader_vertex_attribute.name)
            {
                // copy shader location from reflected layout
                let mut compiled_vertex_attribute = target_vertex_attribute.clone();
                compiled_vertex_attribute.shader_location = shader_vertex_attribute.shader_location;
                compiled_vertex_buffer_descriptor
                    .attributes
                    .push(compiled_vertex_attribute);
            } else {
                fallback_vertex_buffer_descriptor
                    .attributes
                    .push(VertexAttributeDescriptor {
                        name: Default::default(),
                        offset: 0,
                        format: shader_vertex_attribute.format, //TODO: use smallest possible format
                        shader_location: shader_vertex_attribute.shader_location,
                    });
            }
        }

        //TODO: add other buffers (like instancing) here
        let mut vertex_buffer_descriptors = Vec::<VertexBufferDescriptor>::default();
        vertex_buffer_descriptors.push(compiled_vertex_buffer_descriptor);
        if !fallback_vertex_buffer_descriptor.attributes.is_empty() {
            vertex_buffer_descriptors.push(fallback_vertex_buffer_descriptor);
        }
        pipeline_layout.vertex_buffer_descriptors = vertex_buffer_descriptors;
        specialized_descriptor.sample_count = pipeline_specialization.sample_count;
        specialized_descriptor.primitive_topology = pipeline_specialization.primitive_topology;
        specialized_descriptor.index_format = pipeline_specialization.index_format;

        let specialized_pipeline_handle = pipelines.add(specialized_descriptor);
        render_resource_context.create_render_pipeline(
            specialized_pipeline_handle.clone_weak(),
            pipelines.get(&specialized_pipeline_handle).unwrap(),
            &shaders,
        );

        let specialized_pipelines = self
            .specialized_pipelines
            .entry(source_pipeline.clone_weak())
            .or_insert_with(Vec::new);
        let weak_specialized_pipeline_handle = specialized_pipeline_handle.clone_weak();
        specialized_pipelines.push(SpecializedPipeline {
            pipeline: specialized_pipeline_handle,
            specialization: pipeline_specialization.clone(),
        });

        weak_specialized_pipeline_handle
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
