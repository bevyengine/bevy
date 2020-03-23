use super::{
    pipeline::PipelineDescriptor,
    render_resource::{RenderResourceAssignments, RenderResourceAssignmentsId},
    shader::{Shader, ShaderSource},
};
use crate::{
    asset::{AssetStorage, Handle},
    render::{render_graph::RenderGraph, render_resource::resource_name},
};
use legion::prelude::*;
use std::collections::{HashMap, HashSet};

pub struct Renderable {
    pub is_visible: bool,
    pub is_instanced: bool,

    // TODO: make these hidden if possible
    pub pipelines: Vec<Handle<PipelineDescriptor>>,
    pub render_resource_assignments: Option<RenderResourceAssignments>,
}

impl Renderable {
    pub fn instanced() -> Self {
        Renderable {
            is_instanced: true,
            ..Default::default()
        }
    }
}

impl Default for Renderable {
    fn default() -> Self {
        Renderable {
            is_visible: true,
            pipelines: vec![
                Handle::new(0), // TODO: this could be better
            ],
            render_resource_assignments: None,
            is_instanced: false,
        }
    }
}

// TODO: consider using (Typeid, fieldinfo.index) in place of string for hashes
pub struct CompiledShaderMap {
    // TODO: need macro hash lookup
    pub source_to_compiled: HashMap<Handle<Shader>, Vec<(HashSet<String>, Handle<Shader>)>>,
    pub pipeline_to_macro_pipelines:
        HashMap<Handle<PipelineDescriptor>, Vec<(HashSet<String>, Handle<PipelineDescriptor>)>>,
}

impl CompiledShaderMap {
    pub fn new() -> Self {
        CompiledShaderMap {
            source_to_compiled: HashMap::new(),
            pipeline_to_macro_pipelines: HashMap::new(),
        }
    }

    fn update_shader_assignments(
        &mut self,
        render_graph: &mut RenderGraph,
        shader_pipeline_assignments: &mut ShaderPipelineAssignments,
        pipeline_storage: &mut AssetStorage<PipelineDescriptor>,
        shader_storage: &mut AssetStorage<Shader>,
        pipelines: &[Handle<PipelineDescriptor>],
        render_resource_assignments: &RenderResourceAssignments,
    ) {
        for pipeline_handle in pipelines.iter() {
            if let None = self.pipeline_to_macro_pipelines.get(pipeline_handle) {
                self.pipeline_to_macro_pipelines
                    .insert(*pipeline_handle, Vec::new());
            }

            let final_handle = if let Some((_shader_defs, macroed_pipeline_handle)) = self
                .pipeline_to_macro_pipelines
                .get_mut(pipeline_handle)
                .unwrap()
                .iter()
                .find(|(shader_defs, _macroed_pipeline_handle)| {
                    *shader_defs == render_resource_assignments.shader_defs
                }) {
                *macroed_pipeline_handle
            } else {
                let pipeline_descriptor = pipeline_storage.get(pipeline_handle).unwrap();
                let macroed_pipeline_handle = {
                    let mut macroed_vertex_handle = try_compiling_shader_with_macros(
                        self,
                        shader_storage,
                        &render_resource_assignments,
                        &pipeline_descriptor.shader_stages.vertex,
                    );
                    let mut macroed_fragment_handle = pipeline_descriptor
                        .shader_stages
                        .fragment
                        .as_ref()
                        .map(|fragment| {
                            try_compiling_shader_with_macros(
                                self,
                                shader_storage,
                                &render_resource_assignments,
                                fragment,
                            )
                        });

                    if macroed_vertex_handle.is_some() || macroed_fragment_handle.is_some() {
                        let mut macroed_pipeline = pipeline_descriptor.clone();
                        if let Some(vertex) = macroed_vertex_handle.take() {
                            macroed_pipeline.shader_stages.vertex = vertex;
                        }

                        if let Some(fragment) = macroed_fragment_handle.take() {
                            macroed_pipeline.shader_stages.fragment = fragment;
                        }

                        let macroed_pipeline_handle = pipeline_storage.add(macroed_pipeline);
                        // TODO: get correct pass name
                        render_graph
                            .add_pipeline(resource_name::pass::MAIN, macroed_pipeline_handle);
                        macroed_pipeline_handle
                    } else {
                        *pipeline_handle
                    }
                };

                let macro_pipelines = self
                    .pipeline_to_macro_pipelines
                    .get_mut(pipeline_handle)
                    .unwrap();
                macro_pipelines.push((
                    render_resource_assignments.shader_defs.clone(),
                    macroed_pipeline_handle,
                ));
                macroed_pipeline_handle
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
            assignments.push(render_resource_assignments.get_id());
        }
    }
}

pub struct ShaderPipelineAssignments {
    pub assignments: HashMap<Handle<PipelineDescriptor>, Vec<RenderResourceAssignmentsId>>,
}

impl ShaderPipelineAssignments {
    pub fn new() -> Self {
        ShaderPipelineAssignments {
            assignments: HashMap::new(),
        }
    }
}

fn try_compiling_shader_with_macros(
    compiled_shader_map: &mut CompiledShaderMap,
    shader_storage: &mut AssetStorage<Shader>,
    assignments: &RenderResourceAssignments,
    shader_handle: &Handle<Shader>,
) -> Option<Handle<Shader>> {
    if let None = compiled_shader_map.source_to_compiled.get(shader_handle) {
        compiled_shader_map
            .source_to_compiled
            .insert(*shader_handle, Vec::new());
    }

    let compiled_shaders = compiled_shader_map
        .source_to_compiled
        .get_mut(shader_handle)
        .unwrap();
    let shader = shader_storage.get(shader_handle).unwrap();

    // don't produce new shader if the input source is already spirv
    if let ShaderSource::Spirv(_) = shader.source {
        return None;
    }

    if let Some((_shader_defs, compiled_shader)) = compiled_shaders
        .iter()
        .find(|(shader_defs, _shader)| *shader_defs == assignments.shader_defs)
    {
        Some(compiled_shader.clone())
    } else {
        let shader_def_vec = assignments
            .shader_defs
            .iter()
            .cloned()
            .collect::<Vec<String>>();
        let compiled_shader = shader.get_spirv_shader(Some(&shader_def_vec));
        compiled_shaders.push((assignments.shader_defs.clone(), *shader_handle));
        let compiled_shader_handle = shader_storage.add(compiled_shader);
        Some(compiled_shader_handle)
    }
}

pub fn update_shader_assignments(world: &mut World, resources: &mut Resources) {
    // PERF: this seems like a lot of work for things that don't change that often.
    // lots of string + hashset allocations. sees uniform_resource_provider for more context
    {
        let mut shader_pipeline_assignments =
            resources.get_mut::<ShaderPipelineAssignments>().unwrap();
        let mut compiled_shader_map = resources.get_mut::<CompiledShaderMap>().unwrap();
        let mut shader_storage = resources.get_mut::<AssetStorage<Shader>>().unwrap();
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        let mut pipeline_descriptor_storage = resources
            .get_mut::<AssetStorage<PipelineDescriptor>>()
            .unwrap();

        // reset assignments so they are updated every frame
        shader_pipeline_assignments.assignments = HashMap::new();

        // TODO: only update when renderable is changed
        for mut renderable in <Write<Renderable>>::query().iter_mut(world) {
            // skip instanced entities. their batched RenderResourceAssignments will handle shader assignments
            if renderable.is_instanced {
                continue;
            }

            compiled_shader_map.update_shader_assignments(
                &mut render_graph,
                &mut shader_pipeline_assignments,
                &mut pipeline_descriptor_storage,
                &mut shader_storage,
                &renderable.pipelines,
                renderable.render_resource_assignments.as_ref().unwrap(),
            );

            // reset shader_defs so they can be changed next frame
            renderable
                .render_resource_assignments
                .as_mut()
                .unwrap()
                .shader_defs
                .clear();
        }
    }
}
