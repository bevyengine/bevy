use crate::{
    asset::{AssetStorage, Handle},
    render::{render_graph_2::RenderGraph, Shader},
};
use legion::prelude::*;
use std::collections::{HashMap, HashSet};

pub struct Renderable {
    pub is_visible: bool,
    pub shaders: Vec<Handle<Shader>>,
    pub shader_defs: HashSet<String>,
}

impl Default for Renderable {
    fn default() -> Self {
        Renderable {
            is_visible: true,
            shaders: Vec::new(),
            shader_defs: HashSet::new(),
        }
    }
}

pub struct CompiledShaderMap {
    // TODO: need macro hash lookup
    pub source_to_compiled: HashMap<Handle<Shader>, Vec<(HashSet<String>, Handle<Shader>)>>,
}

impl CompiledShaderMap {
    pub fn new() -> Self {
        CompiledShaderMap {
            source_to_compiled: HashMap::new(),
        }
    }
}

pub struct ShaderAssignments {
    pub assignments: HashMap<usize, Vec<Entity>>,
}

impl ShaderAssignments {
    pub fn new() -> Self {
        ShaderAssignments {
            assignments: HashMap::new(),
        }
    }
}

pub fn update_shader_assignments(world: &mut World, render_graph: &mut RenderGraph) {
    // PERF: this seems like a lot of work for things that don't change that often.
    // lots of string + hashset allocations. sees uniform_resource_provider for more context
    {
        let shader_assignments = world.resources.get_mut::<ShaderAssignments>().unwrap();
        let mut compiled_shader_map = world.resources.get_mut::<CompiledShaderMap>().unwrap();
        let mut shader_storage = world.resources.get_mut::<AssetStorage<Shader>>().unwrap();
        for (entity, renderable) in <Read<Renderable>>::query().iter_entities(world) {
            for shader in renderable.shaders.iter() {
                if let None = compiled_shader_map.source_to_compiled.get(shader) {
                    compiled_shader_map
                        .source_to_compiled
                        .insert(shader.clone(), Vec::new());
                }

                let compiled_shaders = compiled_shader_map.source_to_compiled.get_mut(shader).unwrap();
                if let None = compiled_shaders.iter().find(|(shader_defs, _shader)| *shader_defs == renderable.shader_defs) {
                    let shader_resource = shader_storage.get(shader).unwrap();
                    let shader_def_vec = renderable.shader_defs.iter().cloned().collect::<Vec<String>>();
                    let compiled_shader = shader_resource.get_spirv_shader(Some(&shader_def_vec));
                    compiled_shaders.push((renderable.shader_defs.clone(), shader.clone()));
                    let compiled_shader_handle = shader_storage.add(compiled_shader);
                    // TODO: collecting assigments in a map means they won't be removed when the macro changes
                    // TODO: need to somehow grab base shader's pipeline, then copy it 
                    // shader_assignments.assignments.insert()
                }
            }
        }
    }

    // cleanup entity shader_defs so next frame they can be refreshed
    for mut renderable in <Write<Renderable>>::query().iter_mut(world) {
        renderable.shader_defs = HashSet::new();
    }
}
