use crate::{
    pass::{TextureAttachment, PassDescriptor},
    pipeline::{PipelineCompiler, PipelineDescriptor},
    render_graph_2::{Node, ResourceSlots, ResourceSlotInfo},
    render_resource::{RenderResourceAssignments, ResourceInfo},
    renderer_2::RenderContext, draw_target::DrawTarget,
};
use bevy_asset::{AssetStorage, Handle};
use legion::prelude::*;

pub struct PassNode {
    descriptor: PassDescriptor,
    pipelines: Vec<(Handle<PipelineDescriptor>, Vec<Box<dyn DrawTarget>>)>,
    inputs: Vec<ResourceSlotInfo>,
    color_attachment_input_indices: Vec<Option<usize>>,
    depth_stencil_attachment_input_index: Option<usize>,
}

impl PassNode {
    pub fn new(descriptor: PassDescriptor) -> Self {
        let mut inputs = Vec::new();
        let mut color_attachment_input_indices = Vec::new();
        for color_attachment in descriptor.color_attachments.iter() {
            if let TextureAttachment::Input(ref name) = color_attachment.attachment {
                inputs.push(ResourceSlotInfo::new(name.to_string(), ResourceInfo::Texture));
                color_attachment_input_indices.push(Some(inputs.len()));
            } else {
                color_attachment_input_indices.push(None);
            }
        }

        let mut depth_stencil_attachment_input_index = None;
        if let Some(ref depth_stencil_attachment)= descriptor.depth_stencil_attachment {
            if let TextureAttachment::Input(ref name) = depth_stencil_attachment.attachment {
                inputs.push(ResourceSlotInfo::new(name.to_string(), ResourceInfo::Texture));
                depth_stencil_attachment_input_index = Some(inputs.len());
            }
        }

        PassNode {
            descriptor,
            pipelines: Vec::new(),
            inputs,
            color_attachment_input_indices,
            depth_stencil_attachment_input_index,
        }
    }

    pub fn add_pipeline(&mut self, pipeline_handle: Handle<PipelineDescriptor>, draw_targets: Vec<Box<dyn DrawTarget>>) {
        self.pipelines.push((pipeline_handle, draw_targets));
    }
}

impl Node for PassNode {
    fn input(&self) -> &[ResourceSlotInfo] {
        &self.inputs
    }

    fn update(
        &mut self,
        world: &World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
        input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        let render_resource_assignments = resources.get::<RenderResourceAssignments>().unwrap();
        let pipeline_compiler = resources.get::<PipelineCompiler>().unwrap();
        let pipeline_storage = resources.get::<AssetStorage<PipelineDescriptor>>().unwrap();

        for (i, color_attachment) in self.descriptor.color_attachments.iter_mut().enumerate() {
            if let Some(input_index) = self.color_attachment_input_indices[i] {
                color_attachment.attachment = TextureAttachment::RenderResource(input.get(input_index).unwrap());
            }
            
        }

        if let Some(input_index) = self.depth_stencil_attachment_input_index {
            self.descriptor.depth_stencil_attachment.as_mut().unwrap().attachment = TextureAttachment::RenderResource(input.get(input_index).unwrap());
        }

        render_context.begin_pass(
            &self.descriptor,
            &render_resource_assignments,
            &mut |render_pass| {
                for (pipeline_handle, draw_targets) in self.pipelines.iter() {
                    if let Some(compiled_pipelines_iter) =
                        pipeline_compiler.iter_compiled_pipelines(*pipeline_handle)
                    {
                        for compiled_pipeline_handle in compiled_pipelines_iter {
                            let pipeline_descriptor =
                                pipeline_storage.get(compiled_pipeline_handle).unwrap();
                            render_pass.set_pipeline(*compiled_pipeline_handle);
                            for draw_target in draw_targets.iter() {
                                draw_target.draw(
                                    world,
                                    resources,
                                    render_pass,
                                    *compiled_pipeline_handle,
                                    pipeline_descriptor,
                                );
                            }
                        }
                    }
                }
            },
        );
    }
}
