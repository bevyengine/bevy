use super::RenderGraphBuilder;
use crate::{
    asset::{AssetStorage, Handle},
    prelude::{Resources, Shader, World},
    render::{
        draw_target::DrawTarget,
        pass::PassDescriptor,
        pipeline::{PipelineCompiler, PipelineDescriptor},
        render_resource::ResourceProvider,
        renderer::Renderer,
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
}

impl RenderGraph {
    pub fn build<'a, 'b, 'c>(
        &'c mut self,
        pipelines: &'a mut AssetStorage<PipelineDescriptor>,
        shaders: &'b mut AssetStorage<Shader>,
    ) -> RenderGraphBuilder<'a, 'b, 'c> {
        RenderGraphBuilder {
            pipelines,
            shaders,
            current_pass: None,
            render_graph: self,
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

    pub fn setup_pipeline_draw_targets(
        &mut self,
        world: &mut World,
        resources: &Resources,
        renderer: &mut dyn Renderer,
    ) {
        let shader_storage = resources.get::<AssetStorage<Shader>>().unwrap();
        let pipeline_compiler = resources.get::<PipelineCompiler>().unwrap();

        for (pass_name, _pass_descriptor) in self.pass_descriptors.iter() {
            if let Some(pass_pipelines) = self.pass_pipelines.get(pass_name) {
                for pass_pipeline in pass_pipelines.iter() {
                    if let Some(compiled_pipelines_iter) =
                        pipeline_compiler.iter_compiled_pipelines(*pass_pipeline)
                    {
                        for compiled_pipeline_handle in compiled_pipelines_iter {
                            let mut pipeline_storage = resources
                                .get_mut::<AssetStorage<PipelineDescriptor>>()
                                .unwrap();
                            let compiled_pipeline_descriptor =
                                pipeline_storage.get_mut(compiled_pipeline_handle).unwrap();

                            // create wgpu pipeline if it doesn't exist
                            renderer.setup_render_pipeline(
                                *compiled_pipeline_handle,
                                compiled_pipeline_descriptor,
                                &shader_storage,
                            );

                            // setup pipeline draw targets
                            for draw_target_name in compiled_pipeline_descriptor.draw_targets.iter()
                            {
                                let draw_target =
                                    self.draw_targets.get_mut(draw_target_name).unwrap();
                                draw_target.setup(
                                    world,
                                    resources,
                                    renderer,
                                    *compiled_pipeline_handle,
                                    compiled_pipeline_descriptor,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
