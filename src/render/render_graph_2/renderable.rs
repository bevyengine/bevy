use crate::{asset::Handle, render::Shader};

pub struct Renderable {
    pub render: bool,
    pub shaders: Vec<Handle<Shader>>,
}
