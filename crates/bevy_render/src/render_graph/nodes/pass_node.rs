use crate::{
    draw_target::DrawTarget,
    pass::{PassDescriptor, TextureAttachment},
    pipeline::{PipelineCompiler, PipelineDescriptor},
    render_graph::{Node, ResourceSlotInfo, ResourceSlots},
    render_resource::RenderResourceAssignments,
    renderer::RenderContext,
    shader::{FieldBindType, Shader},
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
                inputs.push(ResourceSlotInfo::new(
                    name.to_string(),
                    FieldBindType::Texture,
                ));
                color_attachment_input_indices.push(Some(inputs.len() - 1));
            } else {
                color_attachment_input_indices.push(None);
            }
        }

        let mut depth_stencil_attachment_input_index = None;
        if let Some(ref depth_stencil_attachment) = descriptor.depth_stencil_attachment {
            if let TextureAttachment::Input(ref name) = depth_stencil_attachment.attachment {
                inputs.push(ResourceSlotInfo::new(
                    name.to_string(),
                    FieldBindType::Texture,
                ));
                depth_stencil_attachment_input_index = Some(inputs.len() - 1);
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

    pub fn add_pipeline(
        &mut self,
        pipeline_handle: Handle<PipelineDescriptor>,
        draw_targets: Vec<Box<dyn DrawTarget>>,
    ) {
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
        let pipeline_compiler = resources.get::<PipelineCompiler>().unwrap();
        let pipeline_storage = resources.get::<AssetStorage<PipelineDescriptor>>().unwrap();

        for (i, color_attachment) in self.descriptor.color_attachments.iter_mut().enumerate() {
            if let Some(input_index) = self.color_attachment_input_indices[i] {
                color_attachment.attachment =
                    TextureAttachment::RenderResource(input.get(input_index).unwrap());
            }
        }

        if let Some(input_index) = self.depth_stencil_attachment_input_index {
            self.descriptor
                .depth_stencil_attachment
                .as_mut()
                .unwrap()
                .attachment = TextureAttachment::RenderResource(input.get(input_index).unwrap());
        }

        let shader_storage = resources.get::<AssetStorage<Shader>>().unwrap();
        for (pipeline_handle, draw_targets) in self.pipelines.iter_mut() {
            if let Some(compiled_pipelines_iter) =
                pipeline_compiler.iter_compiled_pipelines(*pipeline_handle)
            {
                for compiled_pipeline_handle in compiled_pipelines_iter {
                    let compiled_pipeline_descriptor =
                        pipeline_storage.get(compiled_pipeline_handle).unwrap();

                    let pipeline_layout = compiled_pipeline_descriptor.get_layout().unwrap();
                    {
                        // TODO: this breaks down in a parallel setting. it needs to change. ideally in a way that
                        // doesn't require modifying RenderResourceAssignments
                        let mut render_resource_assignments =
                            resources.get_mut::<RenderResourceAssignments>().unwrap();
                        for bind_group in pipeline_layout.bind_groups.iter() {
                            render_resource_assignments.update_render_resource_set_id(bind_group);
                        }
                    }

                    render_context.resources().create_render_pipeline(
                        *compiled_pipeline_handle,
                        &compiled_pipeline_descriptor,
                        &shader_storage,
                    );
                    for draw_target in draw_targets.iter_mut() {
                        draw_target.setup(
                            world,
                            resources,
                            render_context,
                            *compiled_pipeline_handle,
                            compiled_pipeline_descriptor,
                        );
                    }
                }
            }
        }

        let render_resource_assignments = resources.get::<RenderResourceAssignments>().unwrap();
        render_context.begin_pass(
            &self.descriptor,
            &render_resource_assignments,
            &mut |render_pass| {
                for (pipeline_handle, draw_targets) in self.pipelines.iter() {
                    if let Some(compiled_pipelines_iter) =
                        pipeline_compiler.iter_compiled_pipelines(*pipeline_handle)
                    {
                        for compiled_pipeline_handle in compiled_pipelines_iter {
                            let compiled_pipeline_descriptor =
                                pipeline_storage.get(compiled_pipeline_handle).unwrap();
                            render_pass.set_pipeline(*compiled_pipeline_handle);
                            for draw_target in draw_targets.iter() {
                                draw_target.draw(
                                    world,
                                    resources,
                                    render_pass,
                                    *compiled_pipeline_handle,
                                    compiled_pipeline_descriptor,
                                );
                            }
                        }
                    }
                }
            },
        );
    }
}
