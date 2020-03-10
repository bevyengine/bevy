use crate::{
    asset::Handle,
    render::{
        draw_target::DrawTarget, pass::PassDescriptor, pipeline::PipelineDescriptor,
        render_resource::ResourceProvider, texture::TextureDescriptor,
    },
};
use std::collections::{HashMap, HashSet};

pub struct RenderGraph {
    pub pipeline_descriptors: HashSet<Handle<PipelineDescriptor>>,
    // TODO: make this ordered
    pub pass_descriptors: HashMap<String, PassDescriptor>,
    pub pass_pipelines: HashMap<String, Vec<Handle<PipelineDescriptor>>>,
    pub resource_providers: Vec<Box<dyn ResourceProvider>>,
    pub queued_textures: Vec<(String, TextureDescriptor)>,
    pub draw_targets: HashMap<String, Box<dyn DrawTarget>>,
}

impl Default for RenderGraph {
    fn default() -> Self {
        RenderGraph {
            pipeline_descriptors: HashSet::new(),
            pass_descriptors: HashMap::new(),
            pass_pipelines: HashMap::new(),
            resource_providers: Vec::new(),
            queued_textures: Vec::new(),
            draw_targets: HashMap::new(),
        }
    }
}

impl RenderGraph {
    pub fn add_pipeline(&mut self, pass: &str, pipeline: Handle<PipelineDescriptor>) {
        self.pipeline_descriptors.insert(pipeline.clone());

        if let None = self.pass_pipelines.get(pass) {
            self.pass_pipelines.insert(pass.to_string(), Vec::new());
        }

        let pass_pipelines = self.pass_pipelines.get_mut(pass).unwrap();
        pass_pipelines.push(pipeline);
    }
}
