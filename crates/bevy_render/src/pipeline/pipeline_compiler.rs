use super::{state_descriptors::PrimitiveTopology, IndexFormat, PipelineDescriptor};
use crate::{
    pipeline::{BindType, VertexBufferLayout},
    renderer::RenderResourceContext,
    shader::{Shader, ShaderError},
};
use bevy_asset::{Assets, Handle};
use bevy_reflect::{Reflect, ReflectDeserialize};
use bevy_utils::{HashMap, HashSet};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
pub struct PipelineSpecialization {
    pub shader_specialization: ShaderSpecialization,
    pub primitive_topology: PrimitiveTopology,
    pub dynamic_bindings: HashSet<String>,
    pub strip_index_format: Option<IndexFormat>,
    pub vertex_buffer_layout: VertexBufferLayout,
    pub sample_count: u32,
}

impl Default for PipelineSpecialization {
    fn default() -> Self {
        Self {
            sample_count: 1,
            strip_index_format: None,
            shader_specialization: Default::default(),
            primitive_topology: Default::default(),
            dynamic_bindings: Default::default(),
            vertex_buffer_layout: Default::default(),
        }
    }
}

impl PipelineSpecialization {
    pub fn empty() -> &'static PipelineSpecialization {
        pub static EMPTY: Lazy<PipelineSpecialization> = Lazy::new(PipelineSpecialization::default);
        &EMPTY
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Default, Reflect, Serialize, Deserialize)]
#[reflect(PartialEq, Serialize, Deserialize)]
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
    specialized_shader_pipelines: HashMap<Handle<Shader>, Vec<Handle<PipelineDescriptor>>>,
    specialized_pipelines: HashMap<Handle<PipelineDescriptor>, Vec<SpecializedPipeline>>,
}

impl PipelineCompiler {
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
        let specialized_vertex_shader = self
            .compile_shader(
                render_resource_context,
                shaders,
                &specialized_descriptor.shader_stages.vertex,
                &pipeline_specialization.shader_specialization,
            )
            .unwrap_or_else(|e| panic_shader_error(e));
        specialized_descriptor.shader_stages.vertex = specialized_vertex_shader.clone_weak();
        let mut specialized_fragment_shader = None;
        specialized_descriptor.shader_stages.fragment = specialized_descriptor
            .shader_stages
            .fragment
            .as_ref()
            .map(|fragment| {
                let shader = self
                    .compile_shader(
                        render_resource_context,
                        shaders,
                        fragment,
                        &pipeline_specialization.shader_specialization,
                    )
                    .unwrap_or_else(|e| panic_shader_error(e));
                specialized_fragment_shader = Some(shader.clone_weak());
                shader
            });

        let mut layout = render_resource_context.reflect_pipeline_layout(
            shaders,
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

        // create a vertex layout that provides all attributes from either the specialized vertex
        // buffers or a zero buffer
        let mut pipeline_layout = specialized_descriptor.layout.as_mut().unwrap();
        // the vertex buffer descriptor of the mesh
        let mesh_vertex_buffer_layout = &pipeline_specialization.vertex_buffer_layout;

        // the vertex buffer descriptor that will be used for this pipeline
        let mut compiled_vertex_buffer_descriptor = VertexBufferLayout {
            step_mode: mesh_vertex_buffer_layout.step_mode,
            stride: mesh_vertex_buffer_layout.stride,
            ..Default::default()
        };

        for shader_vertex_attribute in pipeline_layout.vertex_buffer_descriptors.iter() {
            let shader_vertex_attribute = shader_vertex_attribute
                .attributes
                .get(0)
                .expect("Reflected layout has no attributes.");

            if let Some(target_vertex_attribute) = mesh_vertex_buffer_layout
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
                panic!(
                    "Attribute {} is required by shader, but not supplied by mesh. Either remove the attribute from the shader or supply the attribute ({}) to the mesh.",
                    shader_vertex_attribute.name,
                    shader_vertex_attribute.name,
                );
            }
        }

        // TODO: add other buffers (like instancing) here
        let mut vertex_buffer_descriptors = Vec::<VertexBufferLayout>::default();
        if !pipeline_layout.vertex_buffer_descriptors.is_empty() {
            vertex_buffer_descriptors.push(compiled_vertex_buffer_descriptor);
        }

        pipeline_layout.vertex_buffer_descriptors = vertex_buffer_descriptors;
        specialized_descriptor.multisample.count = pipeline_specialization.sample_count;
        specialized_descriptor.primitive.topology = pipeline_specialization.primitive_topology;
        specialized_descriptor.primitive.strip_index_format =
            pipeline_specialization.strip_index_format;

        let specialized_pipeline_handle = pipelines.add(specialized_descriptor);
        render_resource_context.create_render_pipeline(
            specialized_pipeline_handle.clone_weak(),
            pipelines.get(&specialized_pipeline_handle).unwrap(),
            shaders,
        );

        // track specialized shader pipelines
        self.specialized_shader_pipelines
            .entry(specialized_vertex_shader)
            .or_insert_with(Default::default)
            .push(source_pipeline.clone_weak());
        if let Some(specialized_fragment_shader) = specialized_fragment_shader {
            self.specialized_shader_pipelines
                .entry(specialized_fragment_shader)
                .or_insert_with(Default::default)
                .push(source_pipeline.clone_weak());
        }

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
        self.specialized_pipelines
            .get(&pipeline_handle)
            .map(|compiled_pipelines| {
                compiled_pipelines
                    .iter()
                    .map(|specialized_pipeline| &specialized_pipeline.pipeline)
            })
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

    /// Update specialized shaders and remove any related specialized
    /// pipelines and assets.
    pub fn update_shader(
        &mut self,
        shader: &Handle<Shader>,
        pipelines: &mut Assets<PipelineDescriptor>,
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
