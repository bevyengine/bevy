use super::{state_descriptors::PrimitiveTopology, PipelineDescriptor, VertexBufferDescriptors};
use crate::{
    render_resource::{RenderResourceAssignments, RenderResourceAssignmentsId},
    shader::{Shader, ShaderSource},
    Renderable,
};
use bevy_asset::{Assets, Handle};
use std::collections::{HashMap, HashSet};

use legion::prelude::*;

#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct PipelineSpecialization {
    pub shader_specialization: ShaderSpecialization,
    pub primitive_topology: PrimitiveTopology,
}

#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct ShaderSpecialization {
    pub shader_defs: HashSet<String>,
}

// TODO: consider using (Typeid, fieldinfo.index) in place of string for hashes
#[derive(Default)]
pub struct PipelineCompiler {
    pub shader_source_to_compiled:
        HashMap<Handle<Shader>, Vec<(ShaderSpecialization, Handle<Shader>)>>,
    pub pipeline_source_to_compiled: HashMap<
        Handle<PipelineDescriptor>,
        Vec<(PipelineSpecialization, Handle<PipelineDescriptor>)>,
    >,
}

impl PipelineCompiler {
    fn compile_shader(
        &mut self,
        shaders: &mut Assets<Shader>,
        shader_handle: &Handle<Shader>,
        shader_specialization: &ShaderSpecialization,
    ) -> Handle<Shader> {
        let compiled_shaders = self
            .shader_source_to_compiled
            .entry(*shader_handle)
            .or_insert_with(|| Vec::new());

        let shader = shaders.get(shader_handle).unwrap();

        // don't produce new shader if the input source is already spirv
        if let ShaderSource::Spirv(_) = shader.source {
            return *shader_handle;
        }

        if let Some((_shader_specialization, compiled_shader)) =
            compiled_shaders
                .iter()
                .find(|(current_shader_specialization, _compiled_shader)| {
                    *current_shader_specialization == *shader_specialization
                })
        {
            // if shader has already been compiled with current configuration, use existing shader
            *compiled_shader
        } else {
            // if no shader exists with the current configuration, create new shader and compile
            let shader_def_vec = shader_specialization
                .shader_defs
                .iter()
                .cloned()
                .collect::<Vec<String>>();
            let compiled_shader = shader.get_spirv_shader(Some(&shader_def_vec));
            let compiled_handle = shaders.add(compiled_shader);
            compiled_shaders.push((shader_specialization.clone(), compiled_handle));
            compiled_handle
        }
    }

    fn compile_pipeline(
        &mut self,
        vertex_buffer_descriptors: &VertexBufferDescriptors,
        shaders: &mut Assets<Shader>,
        pipeline_descriptor: &PipelineDescriptor,
        render_resource_assignments: &RenderResourceAssignments,
    ) -> PipelineDescriptor {
        let mut compiled_pipeline_descriptor = pipeline_descriptor.clone();

        compiled_pipeline_descriptor.shader_stages.vertex = self.compile_shader(
            shaders,
            &pipeline_descriptor.shader_stages.vertex,
            &render_resource_assignments
                .pipeline_specialization
                .shader_specialization,
        );
        compiled_pipeline_descriptor.shader_stages.fragment = pipeline_descriptor
            .shader_stages
            .fragment
            .as_ref()
            .map(|fragment| {
                self.compile_shader(
                    shaders,
                    fragment,
                    &render_resource_assignments
                        .pipeline_specialization
                        .shader_specialization,
                )
            });

        compiled_pipeline_descriptor.reflect_layout(
            shaders,
            true,
            Some(vertex_buffer_descriptors),
            Some(render_resource_assignments),
        );

        compiled_pipeline_descriptor.primitive_topology = render_resource_assignments
            .pipeline_specialization
            .primitive_topology;
        compiled_pipeline_descriptor
    }

    fn update_shader_assignments(
        &mut self,
        vertex_buffer_descriptors: &VertexBufferDescriptors,
        shader_pipeline_assignments: &mut PipelineAssignments,
        pipelines: &mut Assets<PipelineDescriptor>,
        shaders: &mut Assets<Shader>,
        pipeline_handles: &[Handle<PipelineDescriptor>],
        render_resource_assignments: &RenderResourceAssignments,
    ) {
        for pipeline_handle in pipeline_handles.iter() {
            if let None = self.pipeline_source_to_compiled.get(pipeline_handle) {
                self.pipeline_source_to_compiled
                    .insert(*pipeline_handle, Vec::new());
            }

            let final_handle = if let Some((_shader_defs, macroed_pipeline_handle)) = self
                .pipeline_source_to_compiled
                .get_mut(pipeline_handle)
                .unwrap()
                .iter()
                .find(|(pipeline_specialization, _macroed_pipeline_handle)| {
                    *pipeline_specialization == render_resource_assignments.pipeline_specialization
                }) {
                *macroed_pipeline_handle
            } else {
                let pipeline_descriptor = pipelines.get(pipeline_handle).unwrap();
                let compiled_pipeline = self.compile_pipeline(
                    vertex_buffer_descriptors,
                    shaders,
                    pipeline_descriptor,
                    render_resource_assignments,
                );
                let compiled_pipeline_handle = pipelines.add(compiled_pipeline);

                let macro_pipelines = self
                    .pipeline_source_to_compiled
                    .get_mut(pipeline_handle)
                    .unwrap();
                macro_pipelines.push((
                    render_resource_assignments.pipeline_specialization.clone(),
                    compiled_pipeline_handle,
                ));
                compiled_pipeline_handle
            };

            // TODO: this will break down if pipeline layout changes. fix this with "auto-layout"
            if let None = shader_pipeline_assignments.assignments.get(&final_handle) {
                shader_pipeline_assignments
                    .assignments
                    .insert(final_handle, Vec::new());
            }

            let assignments = shader_pipeline_assignments
                .assignments
                .get_mut(&final_handle)
                .unwrap();
            assignments.push(render_resource_assignments.id);
        }
    }

    pub fn iter_compiled_pipelines(
        &self,
        pipeline_handle: Handle<PipelineDescriptor>,
    ) -> Option<impl Iterator<Item = &Handle<PipelineDescriptor>>> {
        if let Some(compiled_pipelines) = self.pipeline_source_to_compiled.get(&pipeline_handle) {
            Some(compiled_pipelines.iter().map(|(_, handle)| handle))
        } else {
            None
        }
    }

    pub fn iter_all_compiled_pipelines(&self) -> impl Iterator<Item = &Handle<PipelineDescriptor>> {
        self.pipeline_source_to_compiled
            .values()
            .map(|compiled_pipelines| {
                compiled_pipelines
                    .iter()
                    .map(|(_, pipeline_handle)| pipeline_handle)
            })
            .flatten()
    }
}

#[derive(Default)]
pub struct PipelineAssignments {
    pub assignments: HashMap<Handle<PipelineDescriptor>, Vec<RenderResourceAssignmentsId>>,
}

// TODO: make this a system
pub fn update_shader_assignments(world: &mut World, resources: &Resources) {
    // PERF: this seems like a lot of work for things that don't change that often.
    // lots of string + hashset allocations. sees uniform_resource_provider for more context
    {
        let mut shader_pipeline_assignments = resources.get_mut::<PipelineAssignments>().unwrap();
        let mut pipeline_compiler = resources.get_mut::<PipelineCompiler>().unwrap();
        let mut shaders = resources.get_mut::<Assets<Shader>>().unwrap();
        let vertex_buffer_descriptors = resources.get::<VertexBufferDescriptors>().unwrap();
        let mut pipeline_descriptor_storage = resources
            .get_mut::<Assets<PipelineDescriptor>>()
            .unwrap();

        // reset assignments so they are updated every frame
        shader_pipeline_assignments.assignments = HashMap::new();

        // TODO: only update when renderable is changed
        for mut renderable in <Write<Renderable>>::query().iter_mut(world) {
            // skip instanced entities. their batched RenderResourceAssignments will handle shader assignments
            if renderable.is_instanced {
                continue;
            }

            pipeline_compiler.update_shader_assignments(
                &vertex_buffer_descriptors,
                &mut shader_pipeline_assignments,
                &mut pipeline_descriptor_storage,
                &mut shaders,
                &renderable.pipelines,
                &renderable.render_resource_assignments,
            );

            // reset shader_defs so they can be changed next frame
            renderable
                .render_resource_assignments
                .pipeline_specialization
                .shader_specialization
                .shader_defs
                .clear();
        }
    }
}
