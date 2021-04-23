use std::{any::TypeId, borrow::Cow, sync::Arc};

use bevy_asset::{Assets, Handle};
use bevy_core::Name;
use bevy_ecs::{prelude::World, world::WorldCell};

use crate::{
    pass::{
        LoadOp, Operations, PassDescriptor, RenderPassDepthStencilAttachmentDescriptor,
        TextureAttachment,
    },
    pipeline::{
        BlendFactor, BlendOperation, BlendState, ColorTargetState, ColorWrite, CompareFunction,
        DepthBiasState, DepthStencilState, PipelineCompiler, PipelineDescriptor, PipelineLayout,
        PipelineSpecialization, StencilFaceState, StencilState,
    },
    prelude::{Msaa, Texture},
    render_graph::{
        base::{node::MAIN_RENDER_TEXTURE, texture::MAIN_RENDER_TEXTURE_HANDLE},
        Node, ResourceSlotInfo,
    },
    renderer::{
        BindGroupId, RenderResourceBinding, RenderResourceBindings, RenderResourceContext,
        RenderResourceType,
    },
    shader::{Shader, ShaderStage, ShaderStages},
    texture::{self, TextureFormat},
};

pub struct NamedTextureInput {
    name: Cow<'static, str>,
    handle: Handle<Texture>,
}

impl NamedTextureInput {
    pub fn new(name: Cow<'static, str>, handle: Handle<Texture>) -> Self {
        Self { name, handle }
    }
}

pub struct FullscreenPassNode {
    pass_descriptor: PassDescriptor,
    pipeline_handle: Handle<PipelineDescriptor>,
    inputs: Vec<ResourceSlotInfo>,
    color_attachment_input_indices: Vec<Option<usize>>,
    color_resolve_target_indices: Vec<Option<usize>>,
    default_clear_color_inputs: Vec<usize>,
    specialized_pipeline_handle: Option<Handle<PipelineDescriptor>>,
    bind_groups: Vec<(u32, BindGroupId, Option<Arc<[u32]>>)>,
    render_resource_bindings: RenderResourceBindings,
    texture_inputs: Vec<NamedTextureInput>,
}

impl FullscreenPassNode {
    pub fn new(
        pass_descriptor: PassDescriptor,
        pipeline_handle: Handle<PipelineDescriptor>,
        // texture_inputs: Vec<Cow<'static, str>>,
        texture_inputs: Vec<NamedTextureInput>,
    ) -> Self {
        let mut inputs = Vec::new();
        let mut color_attachment_input_indices = Vec::new();
        let mut color_resolve_target_indices = Vec::new();
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

        // for texture_name in texture_inputs {
        //     inputs.push(ResourceSlotInfo::new(
        //         texture_name,
        //         RenderResourceType::Texture,
        //     ));
        // }

        Self {
            pass_descriptor,
            pipeline_handle,
            inputs,
            color_attachment_input_indices,
            color_resolve_target_indices,
            default_clear_color_inputs: Vec::new(),
            specialized_pipeline_handle: None,
            bind_groups: Vec::new(),
            render_resource_bindings: RenderResourceBindings::default(),
            texture_inputs,
        }
    }
}

impl FullscreenPassNode {
    fn setup_specialized_pipeline(&mut self, world: &mut WorldCell) {
        let mut pipeline_descriptors = world
            .get_resource_mut::<Assets<PipelineDescriptor>>()
            .unwrap();

        if self.specialized_pipeline_handle.is_none() {
            let msaa = world.get_resource::<Msaa>().unwrap();
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
                sample_count: msaa.samples,
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

        let mut pipeline_descriptors = world
            .get_resource_mut::<Assets<PipelineDescriptor>>()
            .unwrap();

        let render_resource_context = world
            .get_resource::<Box<dyn RenderResourceContext>>()
            .unwrap();

        let pipeline_descriptor = pipeline_descriptors
            .get(self.specialized_pipeline_handle.as_ref().unwrap())
            .unwrap();

        // let mut render_resource_bindings =
        //     world.get_resource_mut::<RenderResourceBindings>().unwrap();

        for input in &self.texture_inputs {
            let texture_handle = &input.handle;

            // asset_resource only set after TextureNode has updated once
            if let Some(texture_resource) = render_resource_context
                .get_asset_resource(texture_handle, texture::TEXTURE_ASSET_INDEX)
            {
                let sampler_resource = render_resource_context
                    .get_asset_resource(texture_handle, texture::SAMPLER_ASSET_INDEX)
                    .unwrap();

                let render_resource_name = format!("{}_texture", input.name);
                let sampler_name = format!("{}_sampler", render_resource_name);
                // dbg!(&render_resource_name);

                self.render_resource_bindings.set(
                    &render_resource_name,
                    RenderResourceBinding::Texture(texture_resource.get_texture().unwrap()),
                );
                self.render_resource_bindings.set(
                    &sampler_name,
                    RenderResourceBinding::Sampler(sampler_resource.get_sampler().unwrap()),
                );
            }
        }

        // dbg!(pipeline_descriptor);

        self.bind_groups.clear();
        pipeline_descriptor
            .layout
            .as_ref()
            .unwrap()
            .bind_groups
            .iter()
            .for_each(|bind_group_descriptor| {
                // dbg!(&bind_group_descriptor);
                if let Some(bind_group) = self
                    .render_resource_bindings
                    .update_bind_group(bind_group_descriptor, render_resource_context.as_ref())
                {
                    // dbg!(&bind_group);
                    self.bind_groups.push((
                        bind_group_descriptor.index,
                        bind_group.id,
                        bind_group.dynamic_uniform_indices.clone(),
                    ));
                }
            });
    }

    fn update(
        &mut self,
        world: &bevy_ecs::prelude::World,
        render_context: &mut dyn crate::renderer::RenderContext,
        input: &crate::render_graph::ResourceSlots,
        _output: &mut crate::render_graph::ResourceSlots,
    ) {
        for (i, color_attachment) in self
            .pass_descriptor
            .color_attachments
            .iter_mut()
            .enumerate()
        {
            if let Some(input_index) = self.color_attachment_input_indices[i] {
                color_attachment.attachment =
                    TextureAttachment::Id(input.get(input_index).unwrap().get_texture().unwrap());
            }
            if let Some(input_index) = self.color_resolve_target_indices[i] {
                color_attachment.resolve_target = Some(TextureAttachment::Id(
                    input.get(input_index).unwrap().get_texture().unwrap(),
                ));
            }
        }

        let render_resource_bindings = world.get_resource::<RenderResourceBindings>().unwrap();
        let pipeline_descriptors = world.get_resource::<Assets<PipelineDescriptor>>().unwrap();
        let pipeline_descriptor = pipeline_descriptors
            .get(self.specialized_pipeline_handle.as_ref().unwrap())
            .unwrap();

        // TODO fix better
        if self.bind_groups.len() != self.texture_inputs.len() {
            return;
        }

        render_context.begin_pass(
            &self.pass_descriptor,
            &render_resource_bindings,
            &mut |render_pass| {
                render_pass.set_pipeline(self.specialized_pipeline_handle.as_ref().unwrap());

                self.bind_groups.iter().for_each(
                    |(index, bind_group_id, dynamic_uniform_indices)| {
                        // dbg!();
                        let bind_group_descriptor = pipeline_descriptor
                            .layout
                            .as_ref()
                            .unwrap()
                            .get_bind_group(*index)
                            .unwrap();

                        render_pass.set_bind_group(
                            *index,
                            bind_group_descriptor.id,
                            *bind_group_id,
                            dynamic_uniform_indices.as_deref(),
                        );
                    },
                );

                render_pass.draw(0..6, 0..1);
            },
        );
    }
}
