use super::RenderGraph;
use crate::{
    asset::AssetStorage,
    render::{
        draw_target::DrawTarget, pass::PassDescriptor, pipeline::PipelineDescriptor,
        render_resource::ResourceProvider, texture::TextureDescriptor,
    },
};

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
            let name = pipeline.name.clone();
            let pipeline_descriptor_handle = pipeline_descriptor_storage.add(pipeline);
            pipeline_descriptor_storage
                .set_name(name.unwrap().as_str(), pipeline_descriptor_handle);
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
        let name = pipeline.name.clone();
        let pipeline_descriptor_handle = pipeline_descriptor_storage.add(pipeline);
        pipeline_descriptor_storage.set_name(name.unwrap().as_str(), pipeline_descriptor_handle);
        self.render_graph
            .add_pipeline(pass, pipeline_descriptor_handle);

        self
    }

    pub fn add_resource_provider<T>(mut self, resource_provider: T) -> Self
    where
        T: ResourceProvider + 'static,
    {
        self.render_graph
            .resource_providers
            .push(Box::new(resource_provider));
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
