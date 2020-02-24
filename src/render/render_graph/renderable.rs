use super::PipelineDescriptor;
use crate::{
    asset::{AssetStorage, Handle},
    render::{
        render_graph::{resource_name, RenderGraph},
        Shader, ShaderSource,
    },
};
use legion::prelude::*;
use std::collections::{HashMap, HashSet};

pub struct Renderable {
    pub is_visible: bool,
    pub pipelines: Vec<Handle<PipelineDescriptor>>,
    pub shader_defs: HashSet<String>,
}

impl Default for Renderable {
    fn default() -> Self {
        Renderable {
            is_visible: true,
            pipelines: vec![
                Handle::new(0), // TODO: this could be better
            ],
            shader_defs: HashSet::new(),
        }
    }
}

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
}

pub struct ShaderPipelineAssignments {
    pub assignments: HashMap<Handle<PipelineDescriptor>, Vec<Entity>>,
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
    renderable: &Renderable,
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

    // don't produce new shader if the input source is already spriv
    if let ShaderSource::Spirv(_) = shader.source {
        return None;
    }

    if let Some((_shader_defs, compiled_shader)) = compiled_shaders
        .iter()
        .find(|(shader_defs, _shader)| *shader_defs == renderable.shader_defs)
    {
        Some(compiled_shader.clone())
    } else {
        let shader_def_vec = renderable
            .shader_defs
            .iter()
            .cloned()
            .collect::<Vec<String>>();
        let compiled_shader = shader.get_spirv_shader(Some(&shader_def_vec));
        compiled_shaders.push((renderable.shader_defs.clone(), *shader_handle));
        let compiled_shader_handle = shader_storage.add(compiled_shader);
        Some(compiled_shader_handle)
    }
}
pub fn update_shader_assignments(world: &mut World, render_graph: &mut RenderGraph) {
    // PERF: this seems like a lot of work for things that don't change that often.
    // lots of string + hashset allocations. sees uniform_resource_provider for more context
    {
        let mut shader_pipeline_assignments = world
            .resources
            .get_mut::<ShaderPipelineAssignments>()
            .unwrap();
        let mut compiled_shader_map = world.resources.get_mut::<CompiledShaderMap>().unwrap();
        let mut shader_storage = world.resources.get_mut::<AssetStorage<Shader>>().unwrap();
        let mut pipeline_descriptor_storage = world
            .resources
            .get_mut::<AssetStorage<PipelineDescriptor>>()
            .unwrap();

        // reset assignments so they are updated every frame
        shader_pipeline_assignments.assignments = HashMap::new();

        for (entity, renderable) in <Read<Renderable>>::query().iter_entities(world) {
            for pipeline_handle in renderable.pipelines.iter() {
                if let None = compiled_shader_map
                    .pipeline_to_macro_pipelines
                    .get(pipeline_handle)
                {
                    compiled_shader_map
                        .pipeline_to_macro_pipelines
                        .insert(*pipeline_handle, Vec::new());
                }

                let final_handle = if let Some((_shader_defs, macroed_pipeline_handle)) =
                    compiled_shader_map
                        .pipeline_to_macro_pipelines
                        .get_mut(pipeline_handle)
                        .unwrap()
                        .iter()
                        .find(|(shader_defs, _macroed_pipeline_handle)| {
                            *shader_defs == renderable.shader_defs
                        }) {
                    *macroed_pipeline_handle
                } else {
                    let pipeline_descriptor =
                        pipeline_descriptor_storage.get(pipeline_handle).unwrap();
                    let macroed_pipeline_handle = {
                        let mut macroed_vertex_handle = try_compiling_shader_with_macros(
                            &mut compiled_shader_map,
                            &mut shader_storage,
                            &renderable,
                            &pipeline_descriptor.shader_stages.vertex,
                        );
                        let mut macroed_fragment_handle = pipeline_descriptor
                            .shader_stages
                            .fragment
                            .as_ref()
                            .map(|fragment| {
                                try_compiling_shader_with_macros(
                                    &mut compiled_shader_map,
                                    &mut shader_storage,
                                    &renderable,
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

                            let macroed_pipeline_handle =
                                pipeline_descriptor_storage.add(macroed_pipeline);
                            // TODO: get correct pass name
                            render_graph.add_pipeline(
                                resource_name::pass::MAIN,
                                macroed_pipeline_handle,
                            );
                            macroed_pipeline_handle
                        } else {
                            *pipeline_handle
                        }
                    };

                    let macro_pipelines = compiled_shader_map
                        .pipeline_to_macro_pipelines
                        .get_mut(pipeline_handle)
                        .unwrap();
                    macro_pipelines.push((
                        renderable.shader_defs.clone(),
                        macroed_pipeline_handle,
                    ));
                    macroed_pipeline_handle
                };

                // TODO: this will break down if pipeline layout changes. fix this with "autolayout"
                if let None = shader_pipeline_assignments.assignments.get(&final_handle) {
                    shader_pipeline_assignments
                        .assignments
                        .insert(final_handle, Vec::new());
                }

                let assignments = shader_pipeline_assignments
                    .assignments
                    .get_mut(&final_handle)
                    .unwrap();
                assignments.push(entity);
            }
        }
    }

    // cleanup entity shader_defs so next frame they can be refreshed
    for mut renderable in <Write<Renderable>>::query().iter_mut(world) {
        renderable.shader_defs = HashSet::new();
    }
}
