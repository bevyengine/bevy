use super::RenderGraph;
use crate::{
    {
        draw_target::DrawTarget,
        pass::PassDescriptor,
        pipeline::{PipelineBuilder, PipelineDescriptor},
        render_resource::ResourceProvider,
        texture::TextureDescriptor,
        shader::Shader,
    },
};

use bevy_asset::AssetStorage;

pub struct RenderGraphBuilder<'a, 'b, 'c> {
    pub pipelines: &'a mut AssetStorage<PipelineDescriptor>,
    pub shaders: &'b mut AssetStorage<Shader>,
    pub render_graph: &'c mut RenderGraph,
    pub current_pass: Option<String>,
}

impl<'a, 'b, 'c> RenderGraphBuilder<'a, 'b, 'c> {
    pub fn add_pass(&mut self, name: &str, pass: PassDescriptor) -> &mut Self {
        self.current_pass = Some(name.to_string());
        self.render_graph
            .pass_descriptors
            .insert(name.to_string(), pass);
        self
    }

    pub fn add_pipeline(&mut self, name: &str, build: impl Fn(&mut PipelineBuilder)) -> &mut Self {
        if let Some(ref pass) = self.current_pass {
            let mut builder = PipelineBuilder::new(name, &mut self.shaders);
            build(&mut builder);
            let pipeline = builder.finish();
            let pipeline_descriptor_handle = self.pipelines.add(pipeline);
            self.pipelines.set_name(name, pipeline_descriptor_handle);
            self.render_graph
                .add_pipeline(&pass, pipeline_descriptor_handle);
        }

        self
    }

    pub fn add_pipeline_to_pass(
        &mut self,
        pass: &str,
        name: &str,
        build: impl Fn(&mut PipelineBuilder),
    ) -> &mut Self {
        {
            let mut builder = PipelineBuilder::new(name, &mut self.shaders);
            build(&mut builder);
            let pipeline = builder.finish();
            let pipeline_descriptor_handle = self.pipelines.add(pipeline);
            self.pipelines.set_name(name, pipeline_descriptor_handle);
            self.render_graph
                .add_pipeline(pass, pipeline_descriptor_handle);
        }

        self
    }

    pub fn add_resource_provider<T>(&mut self, resource_provider: T) -> &mut Self
    where
        T: ResourceProvider + Send + Sync + 'static,
    {
        self.render_graph
            .resource_providers
            .push(Box::new(resource_provider));
        self
    }

    pub fn add_texture(&mut self, name: &str, texture_descriptor: TextureDescriptor) -> &mut Self {
        self.render_graph
            .queued_textures
            .push((name.to_string(), texture_descriptor));
        self
    }

    pub fn add_draw_target<T>(&mut self, draw_target: T) -> &mut Self
    where
        T: DrawTarget + Send + Sync + 'static,
    {
        self.render_graph
            .draw_targets
            .insert(draw_target.get_name(), Box::new(draw_target));
        self
    }
}
