use super::RenderGraphBuilder;
use crate::{
    asset::Handle,
    prelude::Resources,
    render::{
        draw_target::DrawTarget,
        pass::PassDescriptor,
        pipeline::{PipelineDescriptor, VertexBufferDescriptor},
        render_resource::ResourceProvider,
        texture::TextureDescriptor,
    },
};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct RenderGraph {
    pub pipeline_descriptors: HashSet<Handle<PipelineDescriptor>>,
    // TODO: make this ordered
    pub pass_descriptors: HashMap<String, PassDescriptor>,
    pub pass_pipelines: HashMap<String, Vec<Handle<PipelineDescriptor>>>,
    pub resource_providers: Vec<Box<dyn ResourceProvider + Send + Sync>>,
    pub queued_textures: Vec<(String, TextureDescriptor)>,
    pub draw_targets: HashMap<String, Box<dyn DrawTarget + Send + Sync>>,
    pub vertex_buffer_descriptors: HashMap<String, VertexBufferDescriptor>,
}

impl RenderGraph {
    pub fn build(self, resources: &mut Resources) -> RenderGraphBuilder {
        RenderGraphBuilder {
            resources,
            current_pass: None,
            render_graph: Some(self),
        }
    }

    pub fn add_pipeline(&mut self, pass: &str, pipeline: Handle<PipelineDescriptor>) {
        self.pipeline_descriptors.insert(pipeline.clone());

        if let None = self.pass_pipelines.get(pass) {
            self.pass_pipelines.insert(pass.to_string(), Vec::new());
        }

        let pass_pipelines = self.pass_pipelines.get_mut(pass).unwrap();
        pass_pipelines.push(pipeline);
    }

    pub fn set_vertex_buffer_descriptor(
        &mut self,
        vertex_buffer_descriptor: VertexBufferDescriptor,
    ) {
        self.vertex_buffer_descriptors.insert(
            vertex_buffer_descriptor.name.to_string(),
            vertex_buffer_descriptor,
        );
    }

    pub fn get_vertex_buffer_descriptor(&self, name: &str) -> Option<&VertexBufferDescriptor> {
        self.vertex_buffer_descriptors.get(name)
    }
}
