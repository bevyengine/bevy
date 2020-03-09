use super::{DrawTarget, PassDescriptor, PipelineDescriptor, ResourceProvider, TextureDescriptor};
use crate::asset::{AssetStorage, Handle};
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

pub struct RenderGraphBuilder {
    render_graph: RenderGraph,
    current_pass: Option<String>,
}

impl RenderGraphBuilder {
    pub fn new() -> Self {
        RenderGraphBuilder {
            render_graph: RenderGraph::default(),
            current_pass: None,
        }
    }

    pub fn add_pass(mut self, name: &str, pass: PassDescriptor) -> Self {
        self.current_pass = Some(name.to_string());
        self.render_graph
            .pass_descriptors
            .insert(name.to_string(), pass);
        self
    }

    pub fn add_pipeline(
        mut self,
        pipeline_descriptor_storage: &mut AssetStorage<PipelineDescriptor>,
        pipeline: PipelineDescriptor,
    ) -> Self {
        if let Some(ref pass) = self.current_pass {
            let pipeline_descriptor_handle = pipeline_descriptor_storage.add(pipeline);
            self.render_graph
                .add_pipeline(&pass, pipeline_descriptor_handle);
        }

        self
    }

    pub fn add_pipeline_to_pass(
        mut self,
        pass: &str,
        pipeline_descriptor_storage: &mut AssetStorage<PipelineDescriptor>,
        pipeline: PipelineDescriptor,
    ) -> Self {
        let pipeline_descriptor_handle = pipeline_descriptor_storage.add(pipeline);
        self.render_graph
            .add_pipeline(pass, pipeline_descriptor_handle);

        self
    }

    pub fn add_resource_provider(mut self, resource_provider: Box<dyn ResourceProvider>) -> Self {
        self.render_graph.resource_providers.push(resource_provider);
        self
    }

    pub fn add_texture(mut self, name: &str, texture_descriptor: TextureDescriptor) -> Self {
        self.render_graph
            .queued_textures
            .push((name.to_string(), texture_descriptor));
        self
    }

    pub fn add_draw_target<T>(mut self, draw_target: T) -> Self
    where
        T: DrawTarget + 'static,
    {
        self.render_graph
            .draw_targets
            .insert(draw_target.get_name(), Box::new(draw_target));
        self
    }

    pub fn build(self) -> RenderGraph {
        self.render_graph
    }
}
