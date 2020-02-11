use crate::{asset::Handle, render::Shader};
use legion::prelude::Entity;
use std::collections::HashSet;

pub struct Renderable {
    pub is_visible: bool,
    pub shaders: Vec<Handle<Shader>>,
}

impl Default for Renderable {
    fn default() -> Self {
        Renderable {
            is_visible: true,
            shaders: Vec::new(),
        }
    }
}

pub struct ShaderAssignments {
    pub assignments: HashSet<usize, Vec<Entity>>,
}
