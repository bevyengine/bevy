use std::borrow::Cow;

use bevy_asset::{Assets, Handle};
use bevy_ecs::{prelude::World, world::WorldCell};

use crate::{
    pass::{PassDescriptor, TextureAttachment},
    pipeline::{
        BindGroupDescriptorId, PipelineCompiler, PipelineDescriptor, PipelineSpecialization,
    },
    render_graph::{Node, ResourceSlotInfo},
    renderer::{
        BindGroupId, RenderResourceBinding, RenderResourceBindings, RenderResourceContext,
        RenderResourceType,
    },
    shader::Shader,
};

#[derive(Debug)]
struct SetBindGroupCommand {
    index: u32,
    descriptor_id: BindGroupDescriptorId,
    bind_group: BindGroupId,
}

/// This node can be used to run a fullscreen pass with a custom pipeline
/// taking optional render textures and samples from previous passes as input.
#[derive(Debug)]
pub struct FullscreenPassNode {
    /// Used to specify attachments and sample count
    pass_descriptor: PassDescriptor,
    /// Shader pipeline that will be used by this fullscreen pass
    pipeline_handle: Handle<PipelineDescriptor>,
    /// Additional Texture, Sampler and Buffer inputs to be used by this fullscreen pass
    inputs: Vec<ResourceSlotInfo>,
    /// Stores indices to quickly look up color attachment inputs
    color_attachment_input_indices: Vec<Option<usize>>,
    /// Stores indices to quickly look up resolve target inputs
    color_resolve_target_input_indices: Vec<Option<usize>>,
    /// Stores indices to quickly look up additional texture inputs
    texture_input_indices: Vec<usize>,
    /// Handle to the compiled pipeline specialization used for rendering
    specialized_pipeline_handle: Option<Handle<PipelineDescriptor>>,
    /// Internal render resource bindings for the additional inputs to this pass
    render_resource_bindings: RenderResourceBindings,
    /// SetBindGroupCommands for this frame, collected during prepare and update
    bind_groups: Vec<SetBindGroupCommand>,
}

impl FullscreenPassNode {
    pub fn new(
        pass_descriptor: PassDescriptor,
        pipeline_handle: Handle<PipelineDescriptor>,
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
            color_resolve_target_input_indices: color_resolve_target_indices,
            texture_input_indices: texture_indices,
            specialized_pipeline_handle: None,
            render_resource_bindings: RenderResourceBindings::default(),
            bind_groups: Vec::new(),
        }
    }
}

impl FullscreenPassNode {
    /// Set up and compile the specialized pipeline to use
    fn setup_specialized_pipeline(&mut self, world: &mut WorldCell) {
        // Get all the necessary resources
        let mut pipeline_descriptors = world
            .get_resource_mut::<Assets<PipelineDescriptor>>()
            .unwrap();

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
            // use the sample count specified in the pass descriptor
            sample_count: self.pass_descriptor.sample_count,
            ..Default::default()
        };

        let specialized_pipeline_handle = if let Some(specialized_pipeline) = pipeline_compiler
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

        render_resource_context.create_render_pipeline(
            specialized_pipeline_handle.clone(),
            &pipeline_descriptor,
            &*shaders,
        );

        self.specialized_pipeline_handle
            .replace(specialized_pipeline_handle);
    }
}

// Update bind groups and collect SetBindGroupCommands in Vec
fn update_bind_groups(
    render_resource_bindings: &mut RenderResourceBindings,
    pipeline_descriptor: &PipelineDescriptor,
    render_resource_context: &dyn RenderResourceContext,
    set_bind_group_commands: &mut Vec<SetBindGroupCommand>,
) {
    // Try to set up the bind group for each descriptor in the pipeline layout
    // Some will be set up later, during update
    for bind_group_descriptor in &pipeline_descriptor.layout.as_ref().unwrap().bind_groups {
        if let Some(bind_group) = render_resource_bindings
            .update_bind_group(bind_group_descriptor, render_resource_context)
        {
            set_bind_group_commands.push(SetBindGroupCommand {
                index: bind_group_descriptor.index,
                descriptor_id: bind_group_descriptor.id,
                bind_group: bind_group.id,
            })
        }
    }
}

impl Node for FullscreenPassNode {
    fn input(&self) -> &[ResourceSlotInfo] {
        &self.inputs
    }

    fn prepare(&mut self, world: &mut World) {
        // Clear previous frame's bind groups
        self.bind_groups.clear();

        let mut world = world.cell();

        // Compile the specialized pipeline
        if self.specialized_pipeline_handle.is_none() {
            self.setup_specialized_pipeline(&mut world);
        }

        // Prepare bind groups
        // Get the necessary resources
        let mut render_resource_bindings =
            world.get_resource_mut::<RenderResourceBindings>().unwrap();

        let pipeline_descriptors = world.get_resource::<Assets<PipelineDescriptor>>().unwrap();

        let render_resource_context = world
            .get_resource::<Box<dyn RenderResourceContext>>()
            .unwrap();

        let pipeline_descriptor = pipeline_descriptors
            .get(self.specialized_pipeline_handle.as_ref().unwrap())
            .unwrap();

        // Do the update
        update_bind_groups(
            &mut render_resource_bindings,
            pipeline_descriptor,
            &**render_resource_context,
            &mut self.bind_groups,
        );
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
            if let Some(input_index) = self.color_resolve_target_input_indices[i] {
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
        // Get the necessary resources
        let pipeline_descriptors = world.get_resource::<Assets<PipelineDescriptor>>().unwrap();
        let pipeline_descriptor = pipeline_descriptors
            .get(self.specialized_pipeline_handle.as_ref().unwrap())
            .unwrap();
        let render_resource_context = render_context.resources_mut();

        // Do the update
        update_bind_groups(
            &mut self.render_resource_bindings,
            pipeline_descriptor,
            render_resource_context,
            &mut self.bind_groups,
        );

        // Check if all bindings are set, will get WGPU error otherwise
        if self.bind_groups.len()
            != pipeline_descriptor
                .layout
                .as_ref()
                .unwrap()
                .bind_groups
                .len()
        {
            panic!("Failed to set all bind groups");
        }

        // Used to lookup texture attachments from the pass_descriptor
        // by name (if needed) during `render_context.begin_pass(..)`
        let render_resource_bindings = world.get_resource::<RenderResourceBindings>().unwrap();

        // Begin actual render pass
        render_context.begin_pass(
            &self.pass_descriptor,
            &render_resource_bindings,
            &mut |render_pass| {
                // Set pipeline
                render_pass.set_pipeline(self.specialized_pipeline_handle.as_ref().unwrap());

                // Set all prepared bind groups
                self.bind_groups.iter().for_each(|command| {
                    render_pass.set_bind_group(
                        command.index,
                        command.descriptor_id,
                        command.bind_group,
                        // Never needed, because no per-object bindings
                        None,
                    );
                });

                // Draw a single triangle without the need for buffers
                // see fullscreen.vert
                render_pass.draw(0..3, 0..1);
            },
        );
    }
}
