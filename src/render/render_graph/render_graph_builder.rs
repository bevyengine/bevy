use super::RenderGraph;
use crate::{
    asset::AssetStorage,
    prelude::{Resources, Shader},
    render::{
        draw_target::DrawTarget,
        pass::PassDescriptor,
        pipeline::{PipelineBuilder, PipelineDescriptor},
        render_resource::ResourceProvider,
        texture::TextureDescriptor,
    },
};

pub struct RenderGraphBuilder<'a> {
    pub render_graph: Option<RenderGraph>,
    pub resources: &'a mut Resources,
    pub current_pass: Option<String>,
}

impl<'a> RenderGraphBuilder<'a> {
    pub fn add_pass(&mut self, name: &str, pass: PassDescriptor) -> &mut Self {
        self.current_pass = Some(name.to_string());
        self.render_graph
            .as_mut()
            .unwrap()
            .pass_descriptors
            .insert(name.to_string(), pass);
        self
    }

    pub fn add_pipeline(&mut self, name: &str, build: impl Fn(&mut PipelineBuilder)) -> &mut Self {
        let mut pipeline_descriptor_storage = self
            .resources
            .get_mut::<AssetStorage<PipelineDescriptor>>()
            .unwrap();
        let mut shader_storage = self.resources.get_mut::<AssetStorage<Shader>>().unwrap();
        if let Some(ref pass) = self.current_pass {
            let mut builder = PipelineBuilder::new(name, &mut shader_storage);
            build(&mut builder);
            let pipeline = builder.finish();
            let pipeline_descriptor_handle = pipeline_descriptor_storage.add(pipeline);
            pipeline_descriptor_storage.set_name(name, pipeline_descriptor_handle);
            self.render_graph
                .as_mut()
                .unwrap()
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
        let mut pipeline_descriptor_storage = self
            .resources
            .get_mut::<AssetStorage<PipelineDescriptor>>()
            .unwrap();
        let mut shader_storage = self.resources.get_mut::<AssetStorage<Shader>>().unwrap();
        let mut builder = PipelineBuilder::new(name, &mut shader_storage);
        build(&mut builder);
        let pipeline = builder.finish();
        let pipeline_descriptor_handle = pipeline_descriptor_storage.add(pipeline);
        pipeline_descriptor_storage.set_name(name, pipeline_descriptor_handle);
        self.render_graph
            .as_mut()
            .unwrap()
            .add_pipeline(pass, pipeline_descriptor_handle);

        self
    }

    pub fn add_resource_provider<T>(&mut self, resource_provider: T) -> &mut Self
    where
        T: ResourceProvider + Send + Sync + 'static,
    {
        self.render_graph
            .as_mut()
            .unwrap()
            .resource_providers
            .push(Box::new(resource_provider));
        self
    }

    pub fn add_texture(&mut self, name: &str, texture_descriptor: TextureDescriptor) -> &mut Self {
        self.render_graph
            .as_mut()
            .unwrap()
            .queued_textures
            .push((name.to_string(), texture_descriptor));
        self
    }

    pub fn add_draw_target<T>(&mut self, draw_target: T) -> &mut Self
    where
        T: DrawTarget + Send + Sync + 'static,
    {
        self.render_graph
            .as_mut()
            .unwrap()
            .draw_targets
            .insert(draw_target.get_name(), Box::new(draw_target));
        self
    }

    pub fn finish(&mut self) -> RenderGraph {
        self.render_graph.take().unwrap()
    }
}
