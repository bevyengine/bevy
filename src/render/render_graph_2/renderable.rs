use crate::{asset::{AssetStorage, Handle}, render::Shader, render::render_graph_2::RenderGraph};
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
        let shader_storage = world.resources.get_mut::<AssetStorage<Shader>>().unwrap();
        for (entity, renderable) in <Read<Renderable>>::query().iter_entities(world) {
            for shader in renderable.shaders.iter() {
                
            }
        }
    }

    // cleanup entity shader_defs so next frame they can be refreshed
    for mut renderable in <Write<Renderable>>::query().iter_mut(world) {
        renderable.shader_defs = HashSet::new();
    }
}