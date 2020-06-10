use crate::{
    draw::{Draw, RenderCommand},
    pass::{PassDescriptor, TextureAttachment},
    render_graph::{Node, ResourceSlotInfo, ResourceSlots},
    render_resource::{EntitiesWaitingForAssets, RenderResourceAssignments, ResourceInfo},
    renderer::RenderContext,
};
use legion::prelude::*;

pub struct MainPassNode {
    descriptor: PassDescriptor,
    inputs: Vec<ResourceSlotInfo>,
    color_attachment_input_indices: Vec<Option<usize>>,
    depth_stencil_attachment_input_index: Option<usize>,
}

impl MainPassNode {
    pub fn new(descriptor: PassDescriptor) -> Self {
        let mut inputs = Vec::new();
        let mut color_attachment_input_indices = Vec::new();
        for color_attachment in descriptor.color_attachments.iter() {
            if let TextureAttachment::Input(ref name) = color_attachment.attachment {
                inputs.push(ResourceSlotInfo::new(
                    name.to_string(),
                    ResourceInfo::Texture(None),
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
                    ResourceInfo::Texture(None),
                ));
                depth_stencil_attachment_input_index = Some(inputs.len() - 1);
            }
        }

        MainPassNode {
            descriptor,
            inputs,
            color_attachment_input_indices,
            depth_stencil_attachment_input_index,
        }
    }
}

impl Node for MainPassNode {
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
        let entities_waiting_for_assets = resources.get::<EntitiesWaitingForAssets>().unwrap();
        let render_resource_assignments = resources.get::<RenderResourceAssignments>().unwrap();

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

        render_context.begin_pass(
            &self.descriptor,
            &render_resource_assignments,
            &mut |render_pass| {
                for (entity, draw) in <Read<Draw>>::query().iter_entities(&world) {
                    if !draw.is_visible || entities_waiting_for_assets.contains(&entity) {
                        continue;
                    }

                    for render_command in draw.render_commands.iter() {
                        match render_command {
                            RenderCommand::SetPipeline { pipeline } => {
                                // TODO: Filter pipelines
                                render_pass.set_pipeline(*pipeline);
                            }
                            RenderCommand::DrawIndexed {
                                base_vertex,
                                indices,
                                instances,
                            } => {
                                render_pass.draw_indexed(
                                    indices.clone(),
                                    *base_vertex,
                                    instances.clone(),
                                );
                            }
                            RenderCommand::SetVertexBuffer {
                                buffer,
                                offset,
                                slot,
                            } => {
                                render_pass.set_vertex_buffer(*slot, *buffer, *offset);
                            }
                            RenderCommand::SetIndexBuffer { buffer, offset } => {
                                render_pass.set_index_buffer(*buffer, *offset);
                            }
                            RenderCommand::SetBindGroup {
                                index,
                                bind_group_descriptor,
                                render_resource_set,
                                dynamic_uniform_indices,
                            } => {
                                render_pass.set_bind_group(
                                    *index,
                                    *bind_group_descriptor,
                                    *render_resource_set,
                                    dynamic_uniform_indices
                                        .as_ref()
                                        .map(|indices| indices.as_slice()),
                                );
                            }
                        }
                    }
                }
            },
        );
    }
}
