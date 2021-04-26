use std::{borrow::Cow, sync::Arc};

use bevy_asset::{Assets, Handle};
use bevy_ecs::{prelude::World, world::WorldCell};

use crate::{
    pass::{PassDescriptor, TextureAttachment},
    pipeline::{BindGroupDescriptor, PipelineCompiler, PipelineDescriptor, PipelineSpecialization},
    prelude::Msaa,
    render_graph::{Node, ResourceSlotInfo},
    renderer::{
        BindGroupId, RenderResourceBinding, RenderResourceBindings, RenderResourceContext,
        RenderResourceType,
    },
    shader::Shader,
};

pub struct FullscreenPassNode {
    pass_descriptor: PassDescriptor,
    pipeline_handle: Handle<PipelineDescriptor>,
    inputs: Vec<ResourceSlotInfo>,
    color_attachment_input_indices: Vec<Option<usize>>,
    color_resolve_target_indices: Vec<Option<usize>>,
    default_clear_color_inputs: Vec<usize>,
    texture_input_indices: Vec<usize>,
    specialized_pipeline_handle: Option<Handle<PipelineDescriptor>>,
    render_resource_bindings: RenderResourceBindings,
}

impl FullscreenPassNode {
    pub fn new(
        pass_descriptor: PassDescriptor,
        pipeline_handle: Handle<PipelineDescriptor>,
        // texture_inputs: Vec<Cow<'static, str>>,
        texture_inputs: Vec<Cow<'static, str>>,
    ) -> Self {
        let mut inputs = Vec::new();
        let mut color_attachment_input_indices = Vec::new();
        let mut color_resolve_target_indices = Vec::new();
        let mut texture_indices = Vec::new();

        for color_attachment in pass_descriptor.color_attachments.iter() {
            if let TextureAttachment::Input(ref name) = color_attachment.attachment {
                color_attachment_input_indices.push(Some(inputs.len()));
                inputs.push(ResourceSlotInfo::new(
                    name.to_string(),
                    RenderResourceType::Texture,
                ));
            } else {
                color_attachment_input_indices.push(None);
            }

            if let Some(TextureAttachment::Input(ref name)) = color_attachment.resolve_target {
                color_resolve_target_indices.push(Some(inputs.len()));
                inputs.push(ResourceSlotInfo::new(
                    name.to_string(),
                    RenderResourceType::Texture,
                ));
            } else {
                color_resolve_target_indices.push(None);
            }
        }

        for texture_name in texture_inputs {
            texture_indices.push(inputs.len());

            let sampler_name = format!("{}_sampler", texture_name);

            inputs.push(ResourceSlotInfo::new(
                texture_name,
                RenderResourceType::Texture,
            ));

            inputs.push(ResourceSlotInfo::new(
                sampler_name,
                RenderResourceType::Sampler,
            ));
        }

        Self {
            pass_descriptor,
            pipeline_handle,
            inputs,
            color_attachment_input_indices,
            color_resolve_target_indices,
            default_clear_color_inputs: Vec::new(),
            texture_input_indices: texture_indices,
            specialized_pipeline_handle: None,
            render_resource_bindings: RenderResourceBindings::default(),
        }
    }
}

impl FullscreenPassNode {
    fn setup_specialized_pipeline(&mut self, world: &mut WorldCell) {
        let mut pipeline_descriptors = world
            .get_resource_mut::<Assets<PipelineDescriptor>>()
            .unwrap();

        if self.specialized_pipeline_handle.is_none() {
            let mut pipeline_compiler = world.get_resource_mut::<PipelineCompiler>().unwrap();
            let mut shaders = world.get_resource_mut::<Assets<Shader>>().unwrap();
            let render_resource_context = world
                .get_resource::<Box<dyn RenderResourceContext>>()
                .unwrap();

            let pipeline_descriptor = pipeline_descriptors
                .get(&self.pipeline_handle)
                .unwrap()
                .clone();

            let pipeline_specialization = PipelineSpecialization {
                sample_count: 1,
                ..Default::default()
            };

            let specialized_pipeline = if let Some(specialized_pipeline) = pipeline_compiler
                .get_specialized_pipeline(&self.pipeline_handle, &pipeline_specialization)
            {
                specialized_pipeline
            } else {
                pipeline_compiler.compile_pipeline(
                    &**render_resource_context,
                    &mut pipeline_descriptors,
                    &mut shaders,
                    &self.pipeline_handle,
                    &pipeline_specialization,
                )
            };

            self.specialized_pipeline_handle
                .replace(specialized_pipeline.clone());

            render_resource_context.create_render_pipeline(
                specialized_pipeline,
                &pipeline_descriptor,
                &*shaders,
            )
        }
    }
}

impl Node for FullscreenPassNode {
    fn input(&self) -> &[ResourceSlotInfo] {
        &self.inputs
    }

    fn prepare(&mut self, world: &mut World) {
        let mut world = world.cell();

        self.setup_specialized_pipeline(&mut world);
    }

    fn update(
        &mut self,
        world: &bevy_ecs::prelude::World,
        render_context: &mut dyn crate::renderer::RenderContext,
        input: &crate::render_graph::ResourceSlots,
        _output: &mut crate::render_graph::ResourceSlots,
    ) {
        // Set color attachments
        for (i, color_attachment) in self
            .pass_descriptor
            .color_attachments
            .iter_mut()
            .enumerate()
        {
            if let Some(input_index) = self.color_attachment_input_indices[i] {
                color_attachment.attachment = TextureAttachment::Id(
                    input
                        .get(input_index)
                        .as_ref()
                        .unwrap()
                        .get_texture()
                        .unwrap(),
                );
            }
            if let Some(input_index) = self.color_resolve_target_indices[i] {
                color_attachment.resolve_target = Some(TextureAttachment::Id(
                    input
                        .get(input_index)
                        .as_ref()
                        .unwrap()
                        .get_texture()
                        .unwrap(),
                ));
            }
        }

        // Prepare RenderResourceBindings
        let render_resource_bindings = world.get_resource::<RenderResourceBindings>().unwrap();

        // Skip first input, the target texture
        for index in &self.texture_input_indices {
            let texture_slot = input.get_slot(*index).unwrap();
            let texture_name = &texture_slot.info.name;
            let texture_id = *texture_slot
                .resource
                .as_ref()
                .unwrap()
                .get_texture()
                .as_ref()
                .unwrap();

            let sampler_slot = input.get_slot(*index + 1).unwrap();
            let sampler_name = &sampler_slot.info.name;
            let sampler_id = *sampler_slot
                .resource
                .as_ref()
                .unwrap()
                .get_sampler()
                .as_ref()
                .unwrap();

            self.render_resource_bindings
                .set(&texture_name, RenderResourceBinding::Texture(texture_id));
            self.render_resource_bindings
                .set(&sampler_name, RenderResourceBinding::Sampler(sampler_id));
        }

        // Prepare bind groups
        type DynamicUniformIndices = Option<Arc<[u32]>>;
        let mut bind_groups: Vec<(&BindGroupDescriptor, BindGroupId, DynamicUniformIndices)> =
            Vec::new();

        let pipeline_descriptors = world.get_resource::<Assets<PipelineDescriptor>>().unwrap();
        let pipeline_descriptor = pipeline_descriptors
            .get(self.specialized_pipeline_handle.as_ref().unwrap())
            .unwrap();
        let render_resource_context = render_context.resources_mut();

        for bind_group_descriptor in &pipeline_descriptor.layout.as_ref().unwrap().bind_groups {
            if let Some(bind_group) = self
                .render_resource_bindings
                .update_bind_group(bind_group_descriptor, render_resource_context)
            {
                bind_groups.push((
                    bind_group_descriptor,
                    bind_group.id,
                    bind_group.dynamic_uniform_indices.clone(),
                ))
            } else {
                panic!("Failed to bind all inputs");
            }
        }

        // Begin actual render pass
        render_context.begin_pass(
            &self.pass_descriptor,
            &render_resource_bindings,
            &mut |render_pass| {
                // Set pipeline
                render_pass.set_pipeline(self.specialized_pipeline_handle.as_ref().unwrap());

                // Set all prepared bind groups
                bind_groups.iter().for_each(
                    |(bind_group_descriptor, bind_group_id, dynamic_uniform_indices)| {
                        render_pass.set_bind_group(
                            bind_group_descriptor.index,
                            bind_group_descriptor.id,
                            *bind_group_id,
                            dynamic_uniform_indices.as_deref(),
                        );
                    },
                );

                // Draw a single triangle without the need for buffers
                // see fullscreen.vert
                render_pass.draw(0..3, 0..1);
            },
        );
    }
}
