use crate::{
    pass::{PassDescriptor, RenderPass, TextureAttachment},
    pipeline::{PipelineAssignments, PipelineCompiler, PipelineDescriptor},
    render_graph::{Node, ResourceSlotInfo, ResourceSlots},
    render_resource::{
        EntitiesWaitingForAssets, EntityRenderResourceAssignments, RenderResourceAssignments,
        ResourceInfo,
    },
    renderer::RenderContext,
    shader::Shader,
    Renderable,
};
use bevy_asset::{Assets, Handle};
use legion::prelude::*;
use std::ops::Range;

pub struct PassNode {
    descriptor: PassDescriptor,
    pipelines: Vec<Handle<PipelineDescriptor>>,
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

        PassNode {
            descriptor,
            pipelines: Vec::new(),
            inputs,
            color_attachment_input_indices,
            depth_stencil_attachment_input_index,
        }
    }

    pub fn add_pipeline(&mut self, pipeline_handle: Handle<PipelineDescriptor>) {
        self.pipelines.push(pipeline_handle);
    }

    fn set_render_resources(
        render_pass: &mut dyn RenderPass,
        pipeline_descriptor: &PipelineDescriptor,
        render_resource_assignments: &RenderResourceAssignments,
    ) -> Option<Range<u32>> {
        let pipeline_layout = pipeline_descriptor.get_layout().unwrap();
        // PERF: vertex buffer lookup comes at a cost when vertex buffers aren't in render_resource_assignments. iterating over render_resource_assignment vertex buffers
        // would likely be faster
        let mut indices = None;
        for (i, vertex_buffer_descriptor) in
            pipeline_layout.vertex_buffer_descriptors.iter().enumerate()
        {
            if let Some((vertex_buffer, index_buffer)) =
                render_resource_assignments.get_vertex_buffer(&vertex_buffer_descriptor.name)
            {
                log::trace!(
                    "set vertex buffer {}: {} ({:?})",
                    i,
                    vertex_buffer_descriptor.name,
                    vertex_buffer
                );
                render_pass.set_vertex_buffer(i as u32, vertex_buffer, 0);
                if let Some(index_buffer) = index_buffer {
                    log::trace!(
                        "set index buffer: {} ({:?})",
                        vertex_buffer_descriptor.name,
                        index_buffer
                    );
                    render_pass.set_index_buffer(index_buffer, 0);
                    render_pass
                        .get_render_context()
                        .resources()
                        .get_resource_info(
                            index_buffer,
                            &mut |resource_info| match resource_info {
                                Some(ResourceInfo::Buffer(Some(buffer_info))) => {
                                    indices = Some(0..(buffer_info.size / 2) as u32)
                                }
                                _ => panic!("expected a buffer type"),
                            },
                        );
                }
            }
        }

        for bind_group in pipeline_layout.bind_groups.iter() {
            if let Some(render_resource_set) =
                render_resource_assignments.get_render_resource_set(bind_group.id)
            {
                render_pass.set_bind_group(bind_group, &render_resource_set);
            }
        }

        indices
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
        let pipelines = resources.get::<Assets<PipelineDescriptor>>().unwrap();
        let shader_pipeline_assignments = resources.get::<PipelineAssignments>().unwrap();
        let entity_render_resource_assignments =
            resources.get::<EntityRenderResourceAssignments>().unwrap();
        let entities_waiting_for_assets = resources.get::<EntitiesWaitingForAssets>().unwrap();
        let mut render_resource_assignments =
            resources.get_mut::<RenderResourceAssignments>().unwrap();

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

        {
            let render_resource_context = render_context.resources();

            // TODO: try merging the two pipeline loops below
            let shaders = resources.get::<Assets<Shader>>().unwrap();
            for pipeline_handle in self.pipelines.iter() {
                if let Some(compiled_pipelines_iter) =
                    pipeline_compiler.iter_compiled_pipelines(*pipeline_handle)
                {
                    for compiled_pipeline_handle in compiled_pipelines_iter {
                        let compiled_pipeline_descriptor =
                            pipelines.get(compiled_pipeline_handle).unwrap();

                        let pipeline_layout = compiled_pipeline_descriptor.get_layout().unwrap();
                        {
                            // TODO: this breaks down in a parallel setting. it needs to change. ideally in a way that
                            // doesn't require modifying RenderResourceAssignments
                            for bind_group in pipeline_layout.bind_groups.iter() {
                                render_resource_assignments
                                    .update_render_resource_set_id(bind_group);
                            }
                        }

                        render_resource_context.create_render_pipeline(
                            *compiled_pipeline_handle,
                            &compiled_pipeline_descriptor,
                            &shaders,
                        );

                        render_resource_context.setup_bind_groups(
                            compiled_pipeline_descriptor,
                            &render_resource_assignments,
                        );
                        let assigned_render_resource_assignments = shader_pipeline_assignments
                            .assignments
                            .get(&compiled_pipeline_handle);
                        if let Some(assigned_render_resource_assignments) =
                            assigned_render_resource_assignments
                        {
                            for assignment_id in assigned_render_resource_assignments.iter() {
                                let entity = entity_render_resource_assignments
                                    .get(*assignment_id)
                                    .unwrap();
                                let renderable =
                                    world.get_component::<Renderable>(*entity).unwrap();
                                if !renderable.is_visible || renderable.is_instanced {
                                    continue;
                                }

                                render_resource_context.setup_bind_groups(
                                    compiled_pipeline_descriptor,
                                    &renderable.render_resource_assignments,
                                );
                            }
                        }
                    }
                }
            }
        }

        render_context.begin_pass(
            &self.descriptor,
            &render_resource_assignments,
            &mut |render_pass| {
                for pipeline_handle in self.pipelines.iter() {
                    if let Some(compiled_pipelines_iter) =
                        pipeline_compiler.iter_compiled_pipelines(*pipeline_handle)
                    {
                        for compiled_pipeline_handle in compiled_pipelines_iter {
                            let compiled_pipeline_descriptor =
                                pipelines.get(compiled_pipeline_handle).unwrap();
                            render_pass.set_pipeline(*compiled_pipeline_handle);

                            // set global render resources
                            Self::set_render_resources(
                                render_pass,
                                compiled_pipeline_descriptor,
                                &render_resource_assignments,
                            );

                            // draw entities assigned to this pipeline
                            let assigned_render_resource_assignments = shader_pipeline_assignments
                                .assignments
                                .get(&compiled_pipeline_handle);

                            if let Some(assigned_render_resource_assignments) =
                                assigned_render_resource_assignments
                            {
                                for assignment_id in assigned_render_resource_assignments.iter() {
                                    // TODO: hopefully legion has better random access apis that are more like queries?
                                    let entity = entity_render_resource_assignments
                                        .get(*assignment_id)
                                        .unwrap();
                                    let renderable =
                                        world.get_component::<Renderable>(*entity).unwrap();
                                    if !renderable.is_visible
                                        || renderable.is_instanced
                                        || entities_waiting_for_assets.contains(entity)
                                    {
                                        continue;
                                    }

                                    // set local render resources
                                    if let Some(indices) = Self::set_render_resources(
                                        render_pass,
                                        compiled_pipeline_descriptor,
                                        &renderable.render_resource_assignments,
                                    ) {
                                        render_pass.draw_indexed(indices, 0, 0..1);
                                    }
                                }
                            }
                        }
                    }
                }
            },
        );
    }
}
