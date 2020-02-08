use crate::render::render_graph_2::RenderPass;
use legion::prelude::World;

// A set of draw calls. ex: get + draw meshes, get + draw instanced meshes, draw ui meshes, etc

// TODO: consider swapping out dyn RenderPass for explicit WgpuRenderPass type to avoid dynamic dispatch
pub type DrawTarget = fn(world: &World, render_pass: &mut dyn RenderPass);
