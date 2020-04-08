use crate::{pipeline::PipelineDescriptor, render_resource::RenderResourceAssignments};
use bevy_asset::Handle;

pub struct Renderable {
    pub is_visible: bool,
    pub is_instanced: bool,

    // TODO: make these hidden if possible
    pub pipelines: Vec<Handle<PipelineDescriptor>>,
    pub render_resource_assignments: RenderResourceAssignments,
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
                Handle::default(),
            ],
            render_resource_assignments: RenderResourceAssignments::default(),
            is_instanced: false,
        }
    }
}
