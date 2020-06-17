use super::{
    state_descriptors::PrimitiveTopology, PipelineDescriptor, RenderPipelines,
    VertexBufferDescriptors,
};
use crate::{
    render_resource::RenderResourceBindings,
    renderer::RenderResourceContext,
    shader::{Shader, ShaderSource},
};
use bevy_asset::{Assets, Handle};
use legion::prelude::*;
use std::collections::{HashMap, HashSet};

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
    pub specialized_pipelines: HashMap<
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
        render_resource_context: &dyn RenderResourceContext,
        pipelines: &mut Assets<PipelineDescriptor>,
        shaders: &mut Assets<Shader>,
        source_pipeline: Handle<PipelineDescriptor>,
        vertex_buffer_descriptors: &VertexBufferDescriptors,
        pipeline_specialization: &PipelineSpecialization,
        render_resource_bindings: &RenderResourceBindings,
    ) -> Handle<PipelineDescriptor> {
        let source_descriptor = pipelines.get(&source_pipeline).unwrap();
        let mut compiled_descriptor = source_descriptor.clone();
        compiled_descriptor.shader_stages.vertex = self.compile_shader(
            shaders,
            &compiled_descriptor.shader_stages.vertex,
            &pipeline_specialization.shader_specialization,
        );
        compiled_descriptor.shader_stages.fragment = compiled_descriptor
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

        compiled_descriptor.reflect_layout(
            shaders,
            true,
            Some(vertex_buffer_descriptors),
            Some(render_resource_bindings),
        );

        compiled_descriptor.primitive_topology = pipeline_specialization.primitive_topology;
        let compiled_pipeline_handle =
            if *pipeline_specialization == PipelineSpecialization::default() {
                pipelines.set(source_pipeline, compiled_descriptor);
                source_pipeline
            } else {
                pipelines.add(compiled_descriptor)
            };

        render_resource_context.create_render_pipeline(
            compiled_pipeline_handle,
            pipelines.get(&compiled_pipeline_handle).unwrap(),
            &shaders,
        );

        let compiled_pipelines = self
            .specialized_pipelines
            .entry(source_pipeline)
            .or_insert_with(|| Vec::new());
        compiled_pipelines.push((pipeline_specialization.clone(), compiled_pipeline_handle));

        compiled_pipeline_handle
    }

    fn compile_render_pipelines(
        &mut self,
        vertex_buffer_descriptors: &VertexBufferDescriptors,
        pipelines: &mut Assets<PipelineDescriptor>,
        shaders: &mut Assets<Shader>,
        render_pipelines: &mut RenderPipelines,
        render_resource_context: &dyn RenderResourceContext,
    ) {
        for render_pipeline in render_pipelines.pipelines.iter_mut() {
            let source_pipeline = render_pipeline.pipeline;
            let compiled_pipeline_handle = if let Some((_shader_defs, compiled_pipeline_handle)) =
                self.specialized_pipelines
                    .get_mut(&source_pipeline)
                    .and_then(|specialized_pipelines| {
                        specialized_pipelines.iter().find(
                            |(pipeline_specialization, _compiled_pipeline_handle)| {
                                *pipeline_specialization == render_pipeline.specialization
                            },
                        )
                    }) {
                *compiled_pipeline_handle
            } else {
                self.compile_pipeline(
                    render_resource_context,
                    pipelines,
                    shaders,
                    source_pipeline,
                    vertex_buffer_descriptors,
                    &render_pipeline.specialization,
                    &render_pipelines.bindings,
                )
            };

            render_pipeline.specialized_pipeline = Some(compiled_pipeline_handle);
        }
    }

    pub fn iter_compiled_pipelines(
        &self,
        pipeline_handle: Handle<PipelineDescriptor>,
    ) -> Option<impl Iterator<Item = &Handle<PipelineDescriptor>>> {
        if let Some(compiled_pipelines) = self.specialized_pipelines.get(&pipeline_handle) {
            Some(compiled_pipelines.iter().map(|(_, handle)| handle))
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
    // pipeline_asset_events: Res<Events<AssetEvent<PipelineDescriptor>>>,
    vertex_buffer_descriptors: Res<VertexBufferDescriptors>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    query: &mut Query<Write<RenderPipelines>>,
) {
    let render_resource_context = &**render_resource_context;
    // let default_specialization = PipelineSpecialization::default();
    // NOTE: this intentionally only handles events that happened prior to this system during this frame. this results in
    // "specialized pipeline" events being ignored.
    // for event in pipeline_asset_events.iter_current_update_events() {
    //     let handle_to_compile = match event {
    //         AssetEvent::Created { handle } => Some(*handle),
    //         AssetEvent::Modified { handle } => {
    //             // TODO: clean up old pipelines
    //             Some(*handle)
    //         }
    //         AssetEvent::Removed { handle } => {
    //             // TODO: clean up old pipelines
    //             None
    //         }
    //     };

    //     if let Some(handle_to_compile) = handle_to_compile {
    //         pipeline_compiler.compile_pipeline(
    //             render_resource_context,
    //             &mut pipelines,
    //             &mut shaders,
    //             handle_to_compile,
    //             &vertex_buffer_descriptors,
    //             &default_specialization,
    //             None,
    //         );
    //     }
    // }

    // TODO: only update when RenderPipelines is changed
    for mut render_pipelines in query.iter_mut(world) {
        pipeline_compiler.compile_render_pipelines(
            &vertex_buffer_descriptors,
            &mut pipelines,
            &mut shaders,
            &mut render_pipelines,
            render_resource_context,
        );

        // reset shader_defs so they can be changed next frame
        for render_pipeline in render_pipelines.pipelines.iter_mut() {
            render_pipeline
                .specialization
                .shader_specialization
                .shader_defs
                .clear();
        }
    }
}
