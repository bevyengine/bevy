use super::{
    state_descriptors::PrimitiveTopology, ComputePipelineDescriptor, IndexFormat, PipelineLayout,
    RenderPipelineDescriptor,
};
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
pub enum PipelineSpecialization {
    Render(RenderPipelineSpecialization),
    Compute(ComputePipelineSpecialization),
}

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
struct SpecializedComputePipeline {
    pipeline: Handle<ComputePipelineDescriptor>,
    specialization: ComputePipelineSpecialization,
}

#[derive(Clone, Eq, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
pub struct RenderPipelineSpecialization {
    pub shader_specialization: ShaderSpecialization,
    pub primitive_topology: PrimitiveTopology,
    pub dynamic_bindings: HashSet<String>,
    pub strip_index_format: Option<IndexFormat>,
    pub vertex_buffer_layout: VertexBufferLayout,
    pub sample_count: u32,
}

impl Default for RenderPipelineSpecialization {
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

impl RenderPipelineSpecialization {
    pub fn empty() -> &'static RenderPipelineSpecialization {
        pub static EMPTY: Lazy<RenderPipelineSpecialization> =
            Lazy::new(RenderPipelineSpecialization::default);
        &EMPTY
    }
}

#[derive(Debug)]
struct SpecializedRenderPipeline {
    pipeline: Handle<RenderPipelineDescriptor>,
    specialization: RenderPipelineSpecialization,
}

#[derive(Clone, Eq, PartialEq, Debug, Default, Reflect, Serialize, Deserialize)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub struct ShaderSpecialization {
    pub shader_defs: HashSet<String>,
}

#[derive(Debug)]
pub struct SpecializedShader {
    pub shader: Handle<Shader>,
    pub specialization: ShaderSpecialization,
}

#[derive(Debug, Default)]
pub struct PipelineCompiler {
    specialized_shaders: HashMap<Handle<Shader>, Vec<SpecializedShader>>,
    specialized_shader_pipelines: HashMap<Handle<Shader>, Vec<Handle<RenderPipelineDescriptor>>>,
    specialized_pipelines:
        HashMap<Handle<RenderPipelineDescriptor>, Vec<SpecializedRenderPipeline>>,
    specialized_shader_compute_pipelines:
        HashMap<Handle<Shader>, Vec<Handle<ComputePipelineDescriptor>>>,
    specialized_compute_pipelines:
        HashMap<Handle<ComputePipelineDescriptor>, Vec<SpecializedComputePipeline>>,
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

    pub fn get_specialized_render_pipeline(
        &self,
        pipeline: &Handle<RenderPipelineDescriptor>,
        specialization: &RenderPipelineSpecialization,
    ) -> Option<Handle<RenderPipelineDescriptor>> {
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

    pub fn get_specialized_compute_pipeline(
        &self,
        pipeline: &Handle<ComputePipelineDescriptor>,
        specialization: &ComputePipelineSpecialization,
    ) -> Option<Handle<ComputePipelineDescriptor>> {
        self.specialized_compute_pipelines
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

    fn set_bind_group_layout_dynamic(
        pipeline_specialization: &PipelineSpecialization,
        layout: &mut PipelineLayout,
    ) {
        // set binding uniforms to dynamic if render resource bindings use dynamic
        for bind_group in layout.bind_groups.iter_mut() {
            let mut binding_changed = false;
            for binding in bind_group.bindings.iter_mut() {
                let pipeline_specialization = match pipeline_specialization {
                    PipelineSpecialization::Compute(compute_ps) => compute_ps
                        .dynamic_bindings
                        .iter()
                        .any(|b| b == &binding.name),
                    PipelineSpecialization::Render(render_ps) => render_ps
                        .dynamic_bindings
                        .iter()
                        .any(|b| b == &binding.name),
                };
                if pipeline_specialization {
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

    pub fn compile_render_pipeline(
        &mut self,
        render_resource_context: &dyn RenderResourceContext,
        pipelines: &mut Assets<RenderPipelineDescriptor>,
        shaders: &mut Assets<Shader>,
        source_pipeline: &Handle<RenderPipelineDescriptor>,
        pipeline_specialization: &RenderPipelineSpecialization,
    ) -> Handle<RenderPipelineDescriptor> {
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
            &shaders,
            &specialized_descriptor.shader_stages,
            true,
        );

        if !pipeline_specialization.dynamic_bindings.is_empty() {
            Self::set_bind_group_layout_dynamic(
                &PipelineSpecialization::Render(pipeline_specialization.clone()),
                &mut layout,
            );
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
            &shaders,
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
        specialized_pipelines.push(SpecializedRenderPipeline {
            pipeline: specialized_pipeline_handle,
            specialization: pipeline_specialization.clone(),
        });

        weak_specialized_pipeline_handle
    }

    pub fn compile_compute_pipeline(
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
            if !pipeline_specialization.dynamic_bindings.is_empty() {
                Self::set_bind_group_layout_dynamic(
                    &PipelineSpecialization::Compute(pipeline_specialization.clone()),
                    &mut layout,
                );
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
        self.specialized_shader_compute_pipelines
            .entry(specialized_compute_shader)
            .or_insert_with(Default::default)
            .push(source_pipeline.clone_weak());

        let specialized_pipelines = self
            .specialized_compute_pipelines
            .entry(source_pipeline.clone_weak())
            .or_insert_with(Vec::new);
        let weak_specialized_pipeline_handle = specialized_pipeline_handle.clone_weak();
        specialized_pipelines.push(SpecializedComputePipeline {
            pipeline: specialized_pipeline_handle,
            specialization: pipeline_specialization.clone(),
        });

        weak_specialized_pipeline_handle
    }

    pub fn iter_compiled_render_pipelines(
        &self,
        pipeline_handle: Handle<RenderPipelineDescriptor>,
    ) -> Option<impl Iterator<Item = &Handle<RenderPipelineDescriptor>>> {
        self.specialized_pipelines
            .get(&pipeline_handle)
            .map(|compiled_pipelines| {
                compiled_pipelines
                    .iter()
                    .map(|specialized_pipeline| &specialized_pipeline.pipeline)
            })
    }

    pub fn iter_compiled_compute_pipelines(
        &self,
        pipeline_handle: Handle<ComputePipelineDescriptor>,
    ) -> Option<impl Iterator<Item = &Handle<ComputePipelineDescriptor>>> {
        self.specialized_compute_pipelines
            .get(&pipeline_handle)
            .map(|compiled_pipelines| {
                compiled_pipelines
                    .iter()
                    .map(|specialized_pipeline| &specialized_pipeline.pipeline)
            })
    }

    pub fn iter_all_compiled_render_pipelines(
        &self,
    ) -> impl Iterator<Item = &Handle<RenderPipelineDescriptor>> {
        self.specialized_pipelines
            .values()
            .map(|compiled_pipelines| {
                compiled_pipelines
                    .iter()
                    .map(|specialized_pipeline| &specialized_pipeline.pipeline)
            })
            .flatten()
    }

    pub fn iter_all_compiled_compute_pipelines(
        &self,
    ) -> impl Iterator<Item = &Handle<ComputePipelineDescriptor>> {
        self.specialized_compute_pipelines
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
        render_pipelines: &mut Assets<RenderPipelineDescriptor>,
        compute_pipelines: &mut Assets<ComputePipelineDescriptor>,
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
                                render_pipelines.remove(p.pipeline);
                            }
                        }
                    }
                }
                if let Some(source_pipelines) = self
                    .specialized_shader_compute_pipelines
                    .remove(&old_handle)
                {
                    // Remove all specialized pipelines from tracking
                    // and asset storage. They will be rebuilt on next
                    // draw.
                    for source_pipeline in source_pipelines {
                        if let Some(specialized_pipelines) =
                            self.specialized_compute_pipelines.remove(&source_pipeline)
                        {
                            for p in specialized_pipelines {
                                compute_pipelines.remove(p.pipeline);
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
