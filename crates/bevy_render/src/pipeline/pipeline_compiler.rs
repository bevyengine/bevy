use super::{state_descriptors::PrimitiveTopology, PipelineDescriptor, VertexBufferDescriptors};
use crate::{
    draw::RenderPipelines,
    renderer::RenderResourceContext,
    shader::{Shader, ShaderSource},
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
        render_pipelines: &RenderPipelines,
    ) -> PipelineDescriptor {
        let mut compiled_pipeline_descriptor = pipeline_descriptor.clone();

        compiled_pipeline_descriptor.shader_stages.vertex = self.compile_shader(
            shaders,
            &pipeline_descriptor.shader_stages.vertex,
            &render_pipelines
                .render_resource_bindings
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
                    &render_pipelines
                        .render_resource_bindings
                        .pipeline_specialization
                        .shader_specialization,
                )
            });

        compiled_pipeline_descriptor.reflect_layout(
            shaders,
            true,
            Some(vertex_buffer_descriptors),
            Some(&render_pipelines.render_resource_bindings),
        );

        compiled_pipeline_descriptor.primitive_topology = render_pipelines
            .render_resource_bindings
            .pipeline_specialization
            .primitive_topology;
        compiled_pipeline_descriptor
    }

    fn compile_pipelines(
        &mut self,
        vertex_buffer_descriptors: &VertexBufferDescriptors,
        pipelines: &mut Assets<PipelineDescriptor>,
        shaders: &mut Assets<Shader>,
        render_pipelines: &mut RenderPipelines,
        render_resource_context: &dyn RenderResourceContext,
    ) {
        for (i, pipeline_handle) in render_pipelines.pipelines.iter().enumerate() {
            if let None = self.pipeline_source_to_compiled.get(pipeline_handle) {
                self.pipeline_source_to_compiled
                    .insert(*pipeline_handle, Vec::new());
            }

            let compiled_pipeline_handle = if let Some((_shader_defs, compiled_pipeline_handle)) =
                self.pipeline_source_to_compiled
                    .get_mut(pipeline_handle)
                    .unwrap()
                    .iter()
                    .find(|(pipeline_specialization, _compiled_pipeline_handle)| {
                        *pipeline_specialization
                            == render_pipelines
                                .render_resource_bindings
                                .pipeline_specialization
                    }) {
                *compiled_pipeline_handle
            } else {
                let pipeline_descriptor = pipelines.get(pipeline_handle).unwrap();
                let compiled_pipeline_descriptor = self.compile_pipeline(
                    vertex_buffer_descriptors,
                    shaders,
                    pipeline_descriptor,
                    render_pipelines,
                );
                let compiled_pipeline_handle = pipelines.add(compiled_pipeline_descriptor);
                render_resource_context.create_render_pipeline(
                    compiled_pipeline_handle,
                    pipelines.get(&compiled_pipeline_handle).unwrap(),
                    &shaders,
                );

                let compiled_pipelines = self
                    .pipeline_source_to_compiled
                    .get_mut(pipeline_handle)
                    .unwrap();
                compiled_pipelines.push((
                    render_pipelines
                        .render_resource_bindings
                        .pipeline_specialization
                        .clone(),
                    compiled_pipeline_handle,
                ));
                compiled_pipeline_handle
            };

            if i == render_pipelines.compiled_pipelines.len() {
                render_pipelines
                    .compiled_pipelines
                    .push(compiled_pipeline_handle);
            } else {
                render_pipelines.compiled_pipelines[i] = compiled_pipeline_handle;
            }
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

// TODO: make this a system
pub fn compile_pipelines_system(
    world: &mut SubWorld,
    mut pipeline_compiler: ResMut<PipelineCompiler>,
    mut shaders: ResMut<Assets<Shader>>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    vertex_buffer_descriptors: Res<VertexBufferDescriptors>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    query: &mut Query<Write<RenderPipelines>>,
) {
    let render_resource_context = &**render_resource_context;

    // TODO: only update when RenderPipelines is changed
    for mut render_pipelines in query.iter_mut(world) {
        pipeline_compiler.compile_pipelines(
            &vertex_buffer_descriptors,
            &mut pipelines,
            &mut shaders,
            &mut render_pipelines,
            render_resource_context,
        );

        // reset shader_defs so they can be changed next frame
        render_pipelines
            .render_resource_bindings
            .pipeline_specialization
            .shader_specialization
            .shader_defs
            .clear();
    }
}
